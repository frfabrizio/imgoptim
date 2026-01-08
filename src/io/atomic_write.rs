use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

/// Atomic write helper: write to a temp file in the same directory then rename.
pub struct AtomicWriter {
    temp: NamedTempFile,
    final_path: PathBuf,
}

impl AtomicWriter {
    pub fn new(final_path: &Path) -> io::Result<Self> {
        let dir = final_path
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "output has no parent"))?;

        let temp = NamedTempFile::new_in(dir)?;
        Ok(Self {
            temp,
            final_path: final_path.to_path_buf(),
        })
    }

    pub fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        use std::io::Write;
        self.temp.write_all(data)?;
        self.temp.flush()?;
        Ok(())
    }

    pub fn commit(self, overwrite: bool) -> io::Result<()> {
        // On Windows, renaming over an existing file can fail => remove first if overwrite.
        if self.final_path.exists() {
            if overwrite {
                fs::remove_file(&self.final_path)?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "output exists",
                ));
            }
        }
        // Persist = atomic rename
        self.temp.persist(&self.final_path).map_err(|e| e.error)?;
        Ok(())
    }
}
