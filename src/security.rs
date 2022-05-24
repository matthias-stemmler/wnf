use std::{alloc, alloc::Layout, ffi::c_void, mem};

use windows::Win32::{
    Foundation::PSID,
    Security::{
        self, CreateWellKnownSid, WinWorldSid, ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, PSECURITY_DESCRIPTOR,
        SECURITY_DESCRIPTOR, SID,
    },
    System::SystemServices::{GENERIC_ALL, SECURITY_DESCRIPTOR_REVISION},
};

use crate::error::SecurityCreateError;

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

    pub(crate) fn create_everyone_generic_all() -> Result<Self, SecurityCreateError> {
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
