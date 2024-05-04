//! Utility functions dealing with privileges

use std::io;

use windows::Win32::Foundation::{BOOL, HANDLE, LUID};
use windows::Win32::Security::{
    LookupPrivilegeValueW, PrivilegeCheck, LUID_AND_ATTRIBUTES, PRIVILEGE_SET, SE_CREATE_PERMANENT_NAME,
    TOKEN_PRIVILEGES_ATTRIBUTES, TOKEN_QUERY,
};
use windows::Win32::System::SystemServices::PRIVILEGE_SET_ALL_NECESSARY;
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// Returns whether the current process has the `SeCreatePermanentPrivilege` privilege
///
/// This privilege is necessary for creating states with the
/// [`StateLifetime::Permanent`](crate::state_name::StateLifetime::Permanent) or
/// [`StateLifetime::Persistent`](crate::state_name::StateLifetime::Persistent) lifetime or with the
/// [`DataScope::Process`](crate::DataScope::Process) scope.
///
/// # Errors
/// Returns an error if checking the privilege fails
pub fn can_create_permanent_shared_objects() -> io::Result<bool> {
    // SAFETY:
    // Calling this function is always safe
    let process_handle = unsafe { GetCurrentProcess() };

    let mut token_handle = HANDLE::default();

    // SAFETY:
    // The pointer in the third argument is valid for writes of `HANDLE` because it comes from a live mutable reference
    unsafe { OpenProcessToken(process_handle, TOKEN_QUERY, &mut token_handle) }?;

    let mut privilege_luid = LUID::default();

    // SAFETY:
    // - The pointer in the second argument points to a valid null-terminated wide string because it comes from a live
    //   `CWideString`
    // - The pointer in the third argument is valid for writes of `LUID` because it comes from a live mutable reference
    unsafe { LookupPrivilegeValueW(None, SE_CREATE_PERMANENT_NAME, &mut privilege_luid) }?;

    let mut privilege_set = PRIVILEGE_SET {
        PrivilegeCount: 1,
        Control: PRIVILEGE_SET_ALL_NECESSARY,
        Privilege: [LUID_AND_ATTRIBUTES {
            Luid: privilege_luid,
            Attributes: TOKEN_PRIVILEGES_ATTRIBUTES::default(),
        }],
    };

    let mut privilege_enabled = BOOL::default();

    // SAFETY:
    // - The pointer in the second argument is valid for writes of `PRIVILEGE_SET` because it comes from a live mutable
    //   reference
    // - The pointer in the third argument is valid for writes of `i32` because it comes from a live mutable reference
    unsafe { PrivilegeCheck(token_handle, &mut privilege_set, &mut privilege_enabled) }?;

    Ok(privilege_enabled.into())
}
