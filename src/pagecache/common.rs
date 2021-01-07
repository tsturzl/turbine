use rio::{Completion, Rio, Ordering};
use std::option::Option;
use tokio::{
    io,
    fs::File,
};
use std::sync::Arc;

pub(crate) const PAGE_SIZE: usize = 4096;
pub(crate) type PageArr = [u8; PAGE_SIZE];

pub struct Page<'p>{
    disk: DiskPtr,
    file: &'p File,
}

impl Page<'_> {
    pub fn new(ptr: DiskPtr, file: &File) -> Page {
        Page{
            disk: ptr,
            file,
        }
    }

    pub async fn read<'a>(&'a mut self, ring: &'a Rio, buf: &'a PageArr) -> io::Result<()> {
        self.disk.read(ring, &self.file, buf).await?;
        Ok(())
    }
}

pub struct DiskPtr(u64);

impl DiskPtr {
    pub async fn new<'a>(ring: &'a Rio, file: &'a File, buf: &'a PageArr) -> io::Result<DiskPtr> {
        let metadata = file.metadata().await?;
        let size = metadata.len();
        let bytes_read = ring.write_at_ordered(file, buf, size, Ordering::Link).await?;
        Ok(DiskPtr(size + bytes_read as u64))
    }

    pub fn load(offset: u64) -> DiskPtr{
        DiskPtr(offset)
    }

    pub fn read<'a>(&'a self, ring: &'a Rio, file: &'a File, buf: &'a PageArr) -> Completion<'a, usize> {
        ring.read_at_ordered(file, buf, self.0, Ordering::Link)
    }
}
