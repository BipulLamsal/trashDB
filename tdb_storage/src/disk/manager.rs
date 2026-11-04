/*
 * Disk Manager
 * Very basic implementation of the disk manager
 * which is able to read and write the page on the disk
 * O_DIRECT is not currently in use, maybe it needs support
 * for iouring as well!
 *
 * */

use std::{
    fs::{File, OpenOptions},
    io,
    os::unix::fs::FileExt,
    path::Path,
};

use crate::disk::{DiskRequest, PAGE_SIZE, RequestKind};

pub struct DiskManager {
    database: File,
}

impl DiskManager {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let database = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        Ok(Self { database })
    }

    pub fn read_page(&self, page_id: usize, buf: &mut [u8]) -> io::Result<()> {
        debug_assert_eq!(buf.len(), PAGE_SIZE);
        let offset = (page_id * PAGE_SIZE) as u64;

        match self.database.read_exact_at(buf, offset) {
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                buf.fill(0);
                Ok(())
            }
            other => other,
        }
    }

    pub fn write_page(&self, page_id: usize, buf: &[u8]) -> io::Result<()> {
        debug_assert_eq!(buf.len(), PAGE_SIZE);
        let offset = page_id as u64 * PAGE_SIZE as u64;
        self.database.write_all_at(buf, offset)
    }

    pub fn perform(&self, request: DiskRequest) -> io::Result<()> {
        match request.kind {
            RequestKind::Read => {
                let mut data = request
                    .data
                    .write()
                    .map_err(|_| io::Error::other("page buffer lock poisoned"))?;
                self.read_page(request.page_id, &mut data[..])
            }
            RequestKind::Write => {
                let data = request
                    .data
                    .read()
                    .map_err(|_| io::Error::other("page buffer lock poisoned"))?;
                self.write_page(request.page_id, &data[..])
            }
        }
    }
}
