use crate::pagecache::common::{Page, PageArr, DiskPtr};

use rio::{Rio, Ordering};
use std::vec::Vec;
use tokio::{
    io,
    sync::Mutex,
    fs::{File, OpenOptions},
};
use std::os::unix::io::{FromRawFd, AsRawFd};

/// Represents a Commit Log
pub struct Log {
    file: File,
    size: u64,
    pub submition_lock: Mutex<()>,
}

impl Log {
    pub async fn new(path: &str) -> io::Result<Log> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(path).await?;

        let metadata = file.metadata().await?;

        Ok(Log {
            file,
            size: metadata.len(),
            submition_lock: Mutex::new(()),
        })
    }

    pub async fn new_page<'a, 'b>(&'a self, ring: &'a Rio, buf: &'b PageArr) -> io::Result<Page<'a>> {
        let ptr = DiskPtr::new(ring, &self.file, buf).await?;
        Ok(Page::new(ptr, &self.file))
    }

    pub async fn flush(&self, ring: &Rio) -> io::Result<()> {
        // rio extracts the RawFd anyway
        unsafe {
            let f = std::fs::File::from_raw_fd(self.file.as_raw_fd());
            ring.fsync_ordered(&f, Ordering::Link).await?;
            Ok(())
        }
    }
}

/// Represents a Commit into a Log
pub struct Commit<'c> {
    pages: Vec<Page<'c>>,
}

impl Commit<'_> {
    pub fn new(pages: Vec<Page>) -> Commit {
        Commit { pages }
    }
}
