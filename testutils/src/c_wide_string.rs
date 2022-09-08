use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;

#[derive(Debug)]
pub struct CWideString(Vec<u16>);

impl CWideString {
    pub fn new<S>(s: &S) -> Self
    where
        S: AsRef<OsStr> + ?Sized,
    {
        Self(OsStr::new(s).encode_wide().chain(Some(0)).collect())
    }

    pub fn as_pcwstr(&self) -> PCWSTR {
        PCWSTR::from_raw(self.0.as_ptr())
    }
}
