use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::{alloc, alloc::Layout, ffi::c_void, io, mem};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, HANDLE, LUID};
use windows::Win32::Security::{
    LookupPrivilegeValueW, PrivilegeCheck, LUID_AND_ATTRIBUTES, PRIVILEGE_SET, TOKEN_PRIVILEGES_ATTRIBUTES, TOKEN_QUERY,
};
use windows::Win32::System::SystemServices::{PRIVILEGE_SET_ALL_NECESSARY, SE_CREATE_PERMANENT_NAME};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::{
    Foundation::PSID,
    Security::{
        self, CreateWellKnownSid, WinWorldSid, ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, PSECURITY_DESCRIPTOR,
        SECURITY_DESCRIPTOR, SID,
    },
    System::SystemServices::{GENERIC_ALL, SECURITY_DESCRIPTOR_REVISION},
};

#[derive(Debug)]
pub(crate) struct SecurityDescriptor {
    raw_security_descriptor: SECURITY_DESCRIPTOR,
    acl_buffer: *mut u8,
    acl_layout: Layout,
}

impl SecurityDescriptor {
    pub(crate) fn as_void_ptr(&self) -> *const c_void {
        &self.raw_security_descriptor as *const SECURITY_DESCRIPTOR as *const c_void
    }

    pub(crate) fn create_everyone_generic_all() -> io::Result<Self> {
        let mut sid = SID::default();
        let psid = PSID(&mut sid as *mut SID as *mut c_void);
        let mut sid_size = mem::size_of::<SID>() as u32;

        unsafe { CreateWellKnownSid(WinWorldSid, None, psid, &mut sid_size) }.ok()?;

        let acl_size =
            mem::size_of::<ACL>() + mem::size_of::<ACCESS_ALLOWED_ACE>() + sid_size as usize - mem::size_of::<u32>();

        let acl_layout = unsafe { Layout::from_size_align_unchecked(acl_size, mem::align_of::<u32>()) };

        let acl_buffer = unsafe { alloc::alloc(acl_layout) };

        unsafe { Security::InitializeAcl(acl_buffer.cast(), acl_size as u32, ACL_REVISION.0) }.ok()?;

        unsafe { Security::AddAccessAllowedAce(acl_buffer.cast(), ACL_REVISION.0, GENERIC_ALL, psid) }.ok()?;

        let mut raw_security_descriptor = SECURITY_DESCRIPTOR::default();
        let psecurity_descriptor =
            PSECURITY_DESCRIPTOR(&mut raw_security_descriptor as *mut SECURITY_DESCRIPTOR as *mut c_void);

        unsafe { Security::InitializeSecurityDescriptor(psecurity_descriptor, SECURITY_DESCRIPTOR_REVISION) }.ok()?;

        unsafe { Security::SetSecurityDescriptorDacl(psecurity_descriptor, true, acl_buffer.cast(), false) }.ok()?;

        Ok(Self {
            raw_security_descriptor,
            acl_buffer,
            acl_layout,
        })
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        unsafe { alloc::dealloc(self.acl_buffer, self.acl_layout) };
    }
}

pub fn can_create_permanent_shared_objects() -> io::Result<bool> {
    let process_handle = unsafe { GetCurrentProcess() };

    let mut token_handle = HANDLE::default();
    let result = unsafe { OpenProcessToken(process_handle, TOKEN_QUERY, &mut token_handle) };
    result.ok()?;

    let mut privilege_luid = LUID::default();
    let result = unsafe {
        LookupPrivilegeValueW(
            None,
            PCWSTR::from_raw(
                OsStr::new(SE_CREATE_PERMANENT_NAME)
                    .encode_wide()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            &mut privilege_luid,
        )
    };
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
    let result = unsafe { PrivilegeCheck(token_handle, &mut privilege_set, &mut privilege_enabled.0) };
    result.ok()?;

    Ok(privilege_enabled.into())
}
