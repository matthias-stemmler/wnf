//! Utility types

#![deny(unsafe_code)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;

/// A null-terminated "wide" (i.e. potentially ill-formed UTF16-encoded) string for use with the Windows API
#[derive(Debug)]
pub(crate) struct CWideString(Vec<u16>);

impl CWideString {
    /// Creates a new [`CWideString`] from the given OS string
    pub(crate) fn new<S>(s: &S) -> Self
    where
        S: AsRef<OsStr> + ?Sized,
    {
        Self(OsStr::new(s).encode_wide().chain(Some(0)).collect())
    }

    /// Returns a raw pointer to the underlying `u16` slice as a [`PCWSTR`] that can be passed to Windows API functions
    ///
    /// The returned pointer is guaranteed to point to a valid null-terminated wide string as long as the instance of
    /// [`CWideString`] is live.
    pub(crate) fn as_pcwstr(&self) -> PCWSTR {
        PCWSTR::from_raw(self.0.as_ptr())
    }
}

#[cfg(test)]
mod tests {
    #![allow(unsafe_code)]

    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::slice;

    use super::*;

    #[test]
    fn as_pcwstr_returns_valid_pointer() {
        let c_wide_string = CWideString::new("test");
        let PCWSTR(ptr) = c_wide_string.as_pcwstr();

        let len = (0..isize::MAX / 2)
            .find(|&idx| {
                // SAFETY: By the guarantees of `as_pcwstr` and because we haven't found a NULL element yet,
                // - both `ptr` and the offset pointer are in bounds of the same allocated object
                // - the computed offset `idx * 2` does not overflow an `isize`
                // - the computed sum does not overflow a `usize`
                let element_ptr = unsafe { ptr.offset(idx) };

                // SAFETY:
                // By the guarantees of `as_pcwstr` and because we haven't found a NULL element yet, `element_ptr`
                // points to a valid `u16`
                let element = unsafe { *element_ptr };

                element == 0
            })
            .unwrap();

        // SAFETY:
        // All safety conditions of `slice::from_raw_parts` follow from the guarantees of `as_pcwstr` and the fact that
        // `len` is the offset relative to `ptr` of the first null element (in particular, `len >= 0`, so the cast to
        // `usize` doesn't change its value)
        let slice = unsafe { slice::from_raw_parts(ptr, len as usize) };

        assert_eq!(OsString::from_wide(slice), "test");
    }
}
