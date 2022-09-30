use std::borrow::Borrow;
use std::ffi::OsStr;
use std::ops::Deref;
use std::os::windows::ffi::OsStrExt;
use std::ptr::NonNull;
use std::str::FromStr;
use std::{io, ptr};

use windows::core::PCWSTR;
use windows::Win32::Security::Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION};
use windows::Win32::Security::PSECURITY_DESCRIPTOR;
use windows::Win32::System::Memory::LocalFree;

#[derive(Debug)]
#[repr(C)]
pub struct SecurityDescriptor {
    _private: [u8; 0],
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        unreachable!("SecurityDescriptor is an opaque type");
    }
}

#[derive(Debug)]
pub struct BoxedSecurityDescriptor {
    ptr: NonNull<SecurityDescriptor>,
}

impl BoxedSecurityDescriptor {
    pub fn create_everyone_generic_all() -> io::Result<Self> {
        "D:(A;;GA;;;WD)".parse()
    }
}

impl FromStr for BoxedSecurityDescriptor {
    type Err = io::Error;

    fn from_str(s: &str) -> io::Result<Self> {
        let mut psecurity_descriptor = PSECURITY_DESCRIPTOR::default();

        let result = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR::from_raw(OsStr::new(s).encode_wide().chain(Some(0)).collect::<Vec<_>>().as_ptr()),
                SDDL_REVISION,
                &mut psecurity_descriptor,
                ptr::null_mut(),
            )
        };

        if !result.as_bool() {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            ptr: NonNull::new(psecurity_descriptor.0 as *mut SecurityDescriptor)
                .expect("ConvertStringSecurityDescriptorToSecurityDescriptorW returned NULL security descriptor"),
        })
    }
}

impl Drop for BoxedSecurityDescriptor {
    fn drop(&mut self) {
        let result = unsafe { LocalFree(self.ptr.as_ptr() as isize) };
        debug_assert_eq!(result, 0);
    }
}

impl Deref for BoxedSecurityDescriptor {
    type Target = SecurityDescriptor;

    fn deref(&self) -> &SecurityDescriptor {
        unsafe { self.ptr.as_ref() }
    }
}

impl Borrow<SecurityDescriptor> for BoxedSecurityDescriptor {
    fn borrow(&self) -> &SecurityDescriptor {
        self
    }
}

#[cfg(feature = "windows-permissions")]
mod impl_windows_permissions {
    use super::*;

    impl Borrow<SecurityDescriptor> for windows_permissions::SecurityDescriptor {
        fn borrow(&self) -> &SecurityDescriptor {
            let ptr = self as *const windows_permissions::SecurityDescriptor as *const SecurityDescriptor;
            unsafe { &*ptr }
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
}
