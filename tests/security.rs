use std::ffi::c_void;

use windows::core::PWSTR;
use windows::Win32::Security::Authorization::{ConvertSecurityDescriptorToStringSecurityDescriptorW, SDDL_REVISION};
use windows::Win32::Security::{DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR};
use windows::Win32::System::Memory::LocalFree;
use wnf::{BoxedSecurityDescriptor, SecurityDescriptor};

#[test]
fn create_everyone_generic_all() {
    // Guard for the null-terminated wide string on the local heap obtained from
    // `ConvertSecurityDescriptorToStringSecurityDescriptorW` below
    struct LocalWideString(PWSTR);

    impl Drop for LocalWideString {
        fn drop(&mut self) {
            // SAFETY:
            // - `self.0` points to a local memory object because it was returned from a successful call to
            //   `ConvertSecurityDescriptorToStringSecurityDescriptorW`
            // - `self.0` has not been freed yet
            unsafe {
                LocalFree(self.0.as_ptr() as isize);
            }
        }
    }

    let security_descriptor = BoxedSecurityDescriptor::create_everyone_generic_all().unwrap();
    let mut sd_wide_string_ptr = PWSTR::null();

    // SAFETY:
    // - The pointer in the first argument is valid for reads of `SecurityDescriptor` because it comes from a live
    //   reference
    // - The pointer in the fourth argument is valid for writes of `PWSTR` because it comes from a live mutable
    //   reference
    let result = unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            PSECURITY_DESCRIPTOR(&*security_descriptor as *const SecurityDescriptor as *mut c_void),
            SDDL_REVISION,
            DACL_SECURITY_INFORMATION.0,
            &mut sd_wide_string_ptr,
            None,
        )
    };

    let successful = result.as_bool();

    assert!(successful);

    // Create a guard to ensure the string is dropped
    let _sd_wide_string = LocalWideString(sd_wide_string_ptr);

    // SAFETY:
    // - The pointer in `sd_string_ptr` is valid for reads up until and including the next `\0` because it was returned
    //   from a successful call to `ConvertSecurityDescriptorToStringSecurityDescriptorW`
    let sd_string = unsafe { sd_wide_string_ptr.to_string() }.unwrap();

    assert_eq!(sd_string, "D:(A;;GA;;;WD)");
}
