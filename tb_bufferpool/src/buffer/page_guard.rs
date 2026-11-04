/* Only buffer pool is able to call this api Its to make lifer easier
 * and not track the pin/unpin.
 * RAII based thread safe guard for bpm.
*/

use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use tdb_storage::disk::PAGE_SIZE;

use crate::buffer::bufferpoolmanager::BufferPoolManger;

pub struct WritePageGuard<'write_lifetime> {
    buffer_pool: &'write_lifetime BufferPoolManger,
    page_id: usize,
    frame_id: usize,
    latch: RwLockWriteGuard<'write_lifetime, Box<[u8; PAGE_SIZE]>>,
}

pub struct ReadPageGuard<'read_lifetime> {
    buffer_pool: &'read_lifetime BufferPoolManger,
    page_id: usize,
    frame_id: usize,
    latch: RwLockReadGuard<'read_lifetime, Box<[u8; PAGE_SIZE]>>,
}

impl<'write_lifetime> WritePageGuard<'write_lifetime> {
    pub(crate) fn new(
        buffer_pool: &'write_lifetime BufferPoolManger,
        page_id: usize,
        frame_id: usize,
        latch: RwLockWriteGuard<'write_lifetime, Box<[u8; PAGE_SIZE]>>,
    ) -> Self {
        Self {
            buffer_pool,
            page_id,
            frame_id,
            latch,
        }
    }

    pub fn page_id(&self) -> usize {
        self.page_id
    }

    pub(crate) fn frame_id(&self) -> usize {
        self.frame_id
    }

    pub fn write(&mut self, data: &[u8]) {
        self.latch.copy_from_slice(data);
    }
}

impl Drop for WritePageGuard<'_> {
    fn drop(&mut self) {
        self.buffer_pool.unpin_page(self.frame_id);
    }
}

impl<'read_lifetime> ReadPageGuard<'read_lifetime> {
    pub(crate) fn new(
        buffer_pool: &'read_lifetime BufferPoolManger,
        page_id: usize,
        frame_id: usize,
        latch: RwLockReadGuard<'read_lifetime, Box<[u8; PAGE_SIZE]>>,
    ) -> Self {
        Self {
            buffer_pool,
            page_id,
            frame_id,
            latch,
        }
    }

    pub fn page_id(&self) -> usize {
        self.page_id
    }

    pub fn data(&self) -> &[u8] {
        &self.latch[..]
    }
}

impl Drop for ReadPageGuard<'_> {
    fn drop(&mut self) {
        self.buffer_pool.unpin_page(self.frame_id);
    }
}
