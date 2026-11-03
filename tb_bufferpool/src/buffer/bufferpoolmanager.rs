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

use crate::replacement::ArcReplacer;

#[derive(Clone, Default)]
struct FrameMetadata {
    page_id: Option<usize>,
    page_count: usize,
    is_dirty: bool,
}

struct BufferPoolState {
    meta: Vec<FrameMetadata>,
    page_map: HashMap<usize, usize>,
    replacer: ArcReplacer,
    page_id: usize,
    disk_manager: DiskManager,
}

struct BufferPoolManger {
    /// Latch for the memory frame
    frames: Vec<Arc<RwLock<Box<[u8; PAGE_SIZE]>>>>,
    state: Mutex<BufferPoolState>,
}

impl BufferPoolManger {
    fn init(capacity: usize, path: PathBuf) -> Self {
        let pre_rolled_frames = vec![Arc::new(RwLock::new(Box::new([0_u8; PAGE_SIZE])))];
        let state = BufferPoolState {
            meta: vec![FrameMetadata::default(); capacity],
            page_map: HashMap::new(),
            replacer: ArcReplacer::init(capacity),
            page_id: 0,
            disk_manager: DiskManager::new(path).expect("no database file  allocated"),
        };

        Self {
            frames: pre_rolled_frames,
            state: Mutex::new(state),
        }
    }

    fn read_page(&mut self, page_id: usize) {}

    fn write_page(&mut self, page_id: usize) {}

    fn delete_page(&mut self, page_id: usize) {}
}
