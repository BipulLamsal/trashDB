/*
 * BufferPool manager manages frames in the memory
 * and related metadata
 * */

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};

use tdb_storage::disk::{DiskManager, PAGE_SIZE};

use crate::{buffer::page_guard::WritePageGuard, replacement::ArcReplacer};

#[derive(Clone, Default)]
struct FrameMetadata {
    page_id: Option<usize>,
    pin_count: usize,
    is_dirty: bool,
}

struct BufferPoolState {
    meta: Vec<FrameMetadata>,
    page_map: HashMap<usize, usize>,
    free_list: Vec<usize>,
    replacer: ArcReplacer,
    page_id: usize,
    disk_manager: DiskManager,
}

pub struct BufferPoolManger {
    /// Latch for the memory frame
    frames: Vec<Arc<RwLock<Box<[u8; PAGE_SIZE]>>>>,
    state: Mutex<BufferPoolState>,
    capacity: usize,
}

impl BufferPoolManger {
    fn init(capacity: usize, path: PathBuf) -> Self {
        let pre_rolled_frames = (0..capacity)
            .map(|_| Arc::new(RwLock::new(Box::new([0_u8; PAGE_SIZE]))))
            .collect::<Vec<_>>();
        let state = BufferPoolState {
            meta: vec![FrameMetadata::default(); capacity],
            page_map: HashMap::new(),
            replacer: ArcReplacer::init(capacity),
            free_list: (0..capacity).collect(),
            page_id: 0,
            disk_manager: DiskManager::new(path).expect("no database file  allocated"),
        };

        Self {
            frames: pre_rolled_frames,
            state: Mutex::new(state),
            capacity,
        }
    }

    /// A new page if created we need to place frame on the
    fn new_page(&self) -> Option<WritePageGuard<'_>> {
        let mut state = self.state.lock().expect("lock failed for metadata");

        let frame_id = match state.free_list.pop() {
            Some(f) => f,
            None => {
                let victim = state.replacer.evict()?; // None if all pinned
                debug_assert_eq!(state.meta[victim].pin_count, 0);

                let old_page_id = state.meta[victim].page_id;
                let was_dirty = state.meta[victim].is_dirty;

                if let Some(old) = old_page_id {
                    if was_dirty {
                        let frame = self.frames[victim].read().unwrap();
                        state
                            .disk_manager
                            .write_page(old, frame.as_ref())
                            .expect("failed to flush dirty page during eviction");
                    }
                    state.page_map.remove(&old);
                }
                victim
            }
        };

        let page_id = state.page_id;
        state.page_id += 1;
        state.page_map.insert(page_id, frame_id);
        state.meta[frame_id] = FrameMetadata {
            page_id: Some(page_id),
            pin_count: 1,
            is_dirty: false,
        };
        state.replacer.record_access(frame_id, page_id);
        state.replacer.set_evictable(frame_id, false);

        let frame_guard = self.frames[frame_id].write().unwrap();

        let page_guard = WritePageGuard::new(self, page_id, frame_id, frame_guard);

        Some(page_guard)
    }

    /// RAII unpin the page automatically when page guard drops
    pub(crate) fn unpin_page(&self, frame_id: usize) {
        let mut state = self.state.lock().expect("lock failed for metadata");

        let pin_count = {
            let meta = &mut state.meta[frame_id];
            debug_assert!(meta.pin_count > 0, "unpin on an unpinned frame");
            meta.pin_count -= 1;
            meta.pin_count
        };

        if pin_count == 0 {
            state.replacer.set_evictable(frame_id, true);
        }
    }

    fn read_page(&mut self, page_id: usize) {}

    fn write_page(&mut self, page_id: usize) {}

    fn delete_page(&mut self, page_id: usize) {}
}

#[cfg(test)]
mod test {
    use crate::buffer::{bufferpoolmanager::BufferPoolManger, page_guard};

    #[test]
    fn test_bpm_new_page() {
        let bpm = std::sync::Arc::new(BufferPoolManger::init(10, "idk.tdb".into()));
        // make the buffer pool full

        let dec = 9;
        for i in 0..10 {
            let page_guard = bpm.new_page().unwrap();
            assert!(page_guard.frame_id() == dec - i);
            assert!(page_guard.page_id() == i);
            // page is created and guard here will unpin it after use
        }

        // now i think things are evictable form 0
        let new_page = bpm.new_page().unwrap();
        assert!(new_page.frame_id() == 9);
    }

    #[test]
    fn test_bpm_when_full() {
        let bpm = std::sync::Arc::new(BufferPoolManger::init(10, "idk.tdb".into()));
        let mut guards = Vec::new();

        for _ in 0..10 {
            let page_guard = bpm.new_page().unwrap();
            guards.push(page_guard);
        }

        let none_page = bpm.new_page();
        assert!(none_page.is_none());
    }
}
