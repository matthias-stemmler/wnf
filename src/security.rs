//! Types dealing with security descriptors

use std::borrow::Borrow;
use std::ffi::c_void;
use std::fmt::{self, Debug, Formatter};
use std::io;
use std::ops::Deref;
use std::ptr::NonNull;
use std::str::FromStr;

use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION};
use windows::Win32::Security::PSECURITY_DESCRIPTOR;

use crate::util::CWideString;

/// A Windows security descriptor
///
/// Since the layout of security descriptors is unstable, this is an *opaque type*, i.e. it is only meant to be used
/// behind a reference or pointer.
///
/// You can configure the security descriptor of a state upon creation through the
/// [`StateCreation::security_descriptor`](crate::manage::StateCreation::security_descriptor) method.
///
/// See RFC [1861-extern-types](https://rust-lang.github.io/rfcs/1861-extern-types.html) for some background on opaque
/// types.
///
/// See [Security Descriptors](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptors) for
/// details about security descriptors.
#[repr(C)]
pub struct SecurityDescriptor {
    _opaque: [u8; 0],
}

impl SecurityDescriptor {
    /// Returns a mutable raw pointer to the security descriptor for use in FFI
    pub(crate) fn as_ptr(&self) -> PSECURITY_DESCRIPTOR {
        PSECURITY_DESCRIPTOR(self as *const Self as *mut c_void)
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        unreachable!("SecurityDescriptor is an opaque type");
    }
}

impl Debug for SecurityDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Hide the `_opaque` field
        f.debug_struct("SecurityDescriptor").finish()
    }
}

/// An owned security descriptor allocated on the local heap
///
/// Unlike [`Box<SecurityDescriptor>`], this allocates memory on the
/// [local heap](https://learn.microsoft.com/en-us/windows/win32/memory/global-and-local-functions).
///
/// There are two ways to create a [`BoxedSecurityDescriptor`]:
/// - via the [`BoxedSecurityDescriptor::create_everyone_generic_all`] method
/// - via the [`FromStr`] implementation of [`BoxedSecurityDescriptor`]
#[derive(Debug)]
pub struct BoxedSecurityDescriptor {
    ptr: NonNull<SecurityDescriptor>,
}

// SAFETY:
// - It is safe to deallocate an object on the local heap that was allocated on a different thread
// - `BoxedSecurityDescriptor` owns a `SecurityDescriptor`, which is `Send`
unsafe impl Send for BoxedSecurityDescriptor {}

// SAFETY:
// `BoxedSecurityDescriptor` derefs to a `SecurityDescriptor`, which is `Sync`
unsafe impl Sync for BoxedSecurityDescriptor {}

impl BoxedSecurityDescriptor {
    /// Creates a security descriptor granting `GENERIC_ALL` access to `Everyone`
    ///
    /// This is the security descriptor used by default when creating states.
    ///
    /// The created security descriptor corresponds to the Security Descriptor String `D:(A;;GA;;;WD)`, meaning it has:
    /// - no owner
    /// - no group
    /// - no System Access Control List (SACL)
    /// - a Discretionary Access Control List (`D` = DACL) with a single Access Control Entry (ACE) granting (`A`) the
    ///   `GENERIC_ALL` access right (`GA`) to `Everyone` (`WD` = World)
    ///
    /// # Errors
    /// Returns an error if creating the security descriptor fails
    pub fn create_everyone_generic_all() -> io::Result<Self> {
        "D:(A;;GA;;;WD)".parse()
    }
}

impl FromStr for BoxedSecurityDescriptor {
    type Err = io::Error;

    /// Parses a [`BoxedSecurityDescriptor`] from a Security Descriptor String
    ///
    /// See
    /// [Security Descriptor String Format](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-string-format)
    /// for details.
    fn from_str(s: &str) -> io::Result<Self> {
        let mut psecurity_descriptor = PSECURITY_DESCRIPTOR::default();
        let string_security_descriptor = CWideString::new(s);

        // SAFETY:
        // - The pointer in the first argument points to a valid null-terminated wide string because it comes from a
        //   live `CWideString`
        // - The pointer in the third argument is valid for writes of `PSECURITY_DESCRIPTOR` because it comes from a
        //   live mutable reference
        // - The pointer in the fourth argument can be `NULL` according to documentation
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                string_security_descriptor.as_pcwstr(),
                SDDL_REVISION,
                &mut psecurity_descriptor,
                None,
            )
        }?;

        Ok(Self {
            ptr: NonNull::new(psecurity_descriptor.0 as *mut SecurityDescriptor)
                .expect("ConvertStringSecurityDescriptorToSecurityDescriptorW returned `NULL` security descriptor"),
        })
    }
}

impl Drop for BoxedSecurityDescriptor {
    fn drop(&mut self) {
        // Note: This can fail, but we have to silently ignore the error because `drop` must not fail

        // SAFETY:
        // - `self.ptr` points to a local memory object because it was returned from
        //   `ConvertStringSecurityDescriptorToSecurityDescriptorW`
        // - `self.ptr` has not been freed yet
        unsafe { LocalFree(HLOCAL(self.ptr.as_ptr() as *mut c_void)) };
    }
}

impl Deref for BoxedSecurityDescriptor {
    type Target = SecurityDescriptor;

    fn deref(&self) -> &SecurityDescriptor {
        // SAFETY:
        // - `self.ptr` is trivially properly aligned as `mem::align_of::<SecurityDescriptor>() == 1`
        // - `self.ptr` points to a valid `SecurityDescriptor` because it was returned from
        //   `ConvertStringSecurityDescriptorToSecurityDescriptorW` and has not been freed yet
        // - the `SecurityDescriptor` pointed to by `self.ptr` is live during the lifetime of the produced reference
        //   because it is not freed before `self` is dropped
        unsafe { self.ptr.as_ref() }
    }
}

impl Borrow<SecurityDescriptor> for BoxedSecurityDescriptor {
    fn borrow(&self) -> &SecurityDescriptor {
        self
    }
}

impl AsRef<SecurityDescriptor> for BoxedSecurityDescriptor {
    fn as_ref(&self) -> &SecurityDescriptor {
        self
    }
}

/// Borrowing security descriptors from
/// [`windows_permissions`](https://docs.rs/windows_permissions/latest/windows_permissions)
#[cfg(feature = "windows_permissions")]
mod impl_windows_permissions {
    use super::*;

    impl Borrow<SecurityDescriptor> for windows_permissions::SecurityDescriptor {
        fn borrow(&self) -> &SecurityDescriptor {
            let ptr = self as *const windows_permissions::SecurityDescriptor as *const SecurityDescriptor;

            // SAFETY:
            // - `ptr` comes from a live reference to a `windows_permissions::SecurityDescriptor` with the same lifetime
            //   as the produced reference
            // - `windows_permissions::SecurityDescriptor` has the same invariants as `SecurityDescriptor`
            unsafe { &*ptr }
        }
    }

    impl AsRef<SecurityDescriptor> for windows_permissions::SecurityDescriptor {
        fn as_ref(&self) -> &SecurityDescriptor {
            self.borrow()
        }
    }

    impl Borrow<SecurityDescriptor> for &windows_permissions::SecurityDescriptor {
        fn borrow(&self) -> &SecurityDescriptor {
            (*self).borrow()
        }
    }

    impl Borrow<SecurityDescriptor> for windows_permissions::LocalBox<windows_permissions::SecurityDescriptor> {
        fn borrow(&self) -> &SecurityDescriptor {
            (**self).borrow()
        }
    }

    impl AsRef<SecurityDescriptor> for windows_permissions::LocalBox<windows_permissions::SecurityDescriptor> {
        fn as_ref(&self) -> &SecurityDescriptor {
            self.borrow()
        }
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_impl_all;

    use super::*;

    #[test]
    fn security_descriptor_is_send_and_sync() {
        assert_impl_all!(SecurityDescriptor: Send, Sync);
    }

    #[test]
    fn boxed_security_descriptor_is_send_and_sync() {
        assert_impl_all!(BoxedSecurityDescriptor: Send, Sync);
    }
}
