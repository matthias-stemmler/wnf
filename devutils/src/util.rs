use std::ffi::OsStr;
use std::io;
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use tempfile::NamedTempFile;
use windows::core::PCWSTR;

/// A null-terminated "wide" (i.e. potentially ill-formed UTF16-encoded) string for use with the Windows API
///
/// See [`wnf::util::CWideString`]
#[derive(Debug)]
pub(crate) struct CWideString(Vec<u16>);

impl CWideString {
    pub(crate) fn new<S>(s: &S) -> Self
    where
        S: AsRef<OsStr> + ?Sized,
    {
        Self(OsStr::new(s).encode_wide().chain(Some(0)).collect())
    }

    pub(crate) fn as_pcwstr(&self) -> PCWSTR {
        PCWSTR::from_raw(self.0.as_ptr())
    }
}

/// A temporary file
#[derive(Debug)]
pub(crate) struct TempFile(NamedTempFile);

impl TempFile {
    /// Creates a new temporary file
    ///
    /// The file is deleted when the [`TempFile`] instance is dropped.
    pub(crate) fn new() -> io::Result<Self> {
        Ok(Self(NamedTempFile::new()?))
    }

    /// Returns the path of the temporary file
    pub(crate) fn path(&self) -> &Path {
        self.0.path()
    }

    /// Reads the contents of the temporary file into the given writer
    pub(crate) fn read_to<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        io::copy(self.0.as_file_mut(), writer)?;
        Ok(())
    }
}
