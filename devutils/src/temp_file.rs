use std::io;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

#[derive(Debug)]
pub struct TempFile(NamedTempFile);

impl TempFile {
    pub fn new() -> io::Result<Self> {
        Ok(Self(NamedTempFile::new()?))
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }

    pub fn read_to<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        io::copy(self.0.as_file_mut(), writer)?;
        Ok(())
    }
}
