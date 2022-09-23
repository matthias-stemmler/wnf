use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::str::FromStr;
use std::{ffi::c_void, io, ptr};

use windows::core::PCWSTR;
use windows::Win32::Security::Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION};
use windows::Win32::Security::PSECURITY_DESCRIPTOR;
use windows::Win32::System::Memory::LocalFree;

/// # Safety
/// It is safe to implement this trait for a type `T` if and only if the raw pointer returned from
/// [`as_raw_security_descriptor`](Self::as_raw_security_descriptor) for an instance of `T` points to a security
/// descriptor that is valid during the lifetime of the instance of `T`.
pub unsafe trait AsRawSecurityDescriptor {
    fn as_raw_security_descriptor(&self) -> *mut c_void;
}

unsafe impl<SD> AsRawSecurityDescriptor for &SD
where
    SD: AsRawSecurityDescriptor,
{
    fn as_raw_security_descriptor(&self) -> *mut c_void {
        SD::as_raw_security_descriptor(self)
    }
}

#[derive(Debug)]
pub struct SecurityDescriptor(PSECURITY_DESCRIPTOR);

impl SecurityDescriptor {
    pub fn create_everyone_generic_all() -> io::Result<Self> {
        "D:(A;;GA;;;WD)".parse()
    }

    unsafe fn from_raw(sd_ptr: *mut c_void) -> Self {
        Self(PSECURITY_DESCRIPTOR(sd_ptr))
    }
}

impl FromStr for SecurityDescriptor {
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

        Ok(Self(psecurity_descriptor))
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        unsafe { LocalFree(self.as_raw_security_descriptor() as isize) };
    }
}

unsafe impl AsRawSecurityDescriptor for SecurityDescriptor {
    fn as_raw_security_descriptor(&self) -> *mut c_void {
        let PSECURITY_DESCRIPTOR(sd_ptr) = self.0;
        sd_ptr
    }
}

#[cfg(feature = "windows-permissions")]
mod impl_windows_permissions {
    use super::*;

    unsafe impl AsRawSecurityDescriptor for windows_permissions::SecurityDescriptor {
        fn as_raw_security_descriptor(&self) -> *mut c_void {
            self as *const Self as *mut c_void
        }
    }

    unsafe impl AsRawSecurityDescriptor for windows_permissions::LocalBox<windows_permissions::SecurityDescriptor> {
        fn as_raw_security_descriptor(&self) -> *mut c_void {
            (**self).as_raw_security_descriptor()
        }
    }
}
