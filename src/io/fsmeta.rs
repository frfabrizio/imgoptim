use std::io;
use std::path::Path;

use filetime::{set_file_times, FileTime};

pub fn preserve_timestamps(src: &Path, dst: &Path) -> io::Result<()> {
    let meta = std::fs::metadata(src)?;
    let mtime = FileTime::from_last_modification_time(&meta);
    let atime = FileTime::from_last_access_time(&meta);
    set_file_times(dst, atime, mtime)?;
    Ok(())
}
