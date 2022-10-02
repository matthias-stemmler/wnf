//! Utility functions dealing with privileges

use std::io;

use windows::Win32::Foundation::{BOOL, HANDLE, LUID};
use windows::Win32::Security::{
    LookupPrivilegeValueW, PrivilegeCheck, LUID_AND_ATTRIBUTES, PRIVILEGE_SET, TOKEN_PRIVILEGES_ATTRIBUTES, TOKEN_QUERY,
};
use windows::Win32::System::SystemServices::{PRIVILEGE_SET_ALL_NECESSARY, SE_CREATE_PERMANENT_NAME};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use crate::util::CWideString;

/// Returns whether the `SeCreatePermanentPrivilege` privilege is enabled in the access token associated with the
/// current process
///
/// This privilege is necessary for creating WNF states with the [`WnfStateNameLifetime::Permanent`] or
/// [`WnfStateNameLifetime::Persistent`] lifetimes.
pub fn can_create_permanent_shared_objects() -> io::Result<bool> {
    // SAFETY:
    // Calling this function is always safe
    let process_handle = unsafe { GetCurrentProcess() };

    let mut token_handle = HANDLE::default();

    // SAFETY:
    // The pointer in the third argument is valid for writes of `HANDLE` because it comes from a live mutable reference
    let result = unsafe { OpenProcessToken(process_handle, TOKEN_QUERY, &mut token_handle) };

    result.ok()?;

    let mut privilege_luid = LUID::default();
    let privilege_name = CWideString::new(SE_CREATE_PERMANENT_NAME);

    // SAFETY:
    // - The pointer in the second argument points to a valid null-terminated wide string because it comes from a live
    //   `CWideString`
    // - The pointer in the third argument is valid for writes of `LUID` because it comes from a live mutable reference
    let result = unsafe { LookupPrivilegeValueW(None, privilege_name.as_pcwstr(), &mut privilege_luid) };

    result.ok()?;

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
    let result = unsafe { PrivilegeCheck(token_handle, &mut privilege_set, &mut privilege_enabled.0) };

    result.ok()?;

    Ok(privilege_enabled.into())
}
