pub mod manager;
pub mod scheduler;

use std::{
    io,
    sync::{Arc, RwLock, mpsc::SyncSender},
};

pub use manager::DiskManager;
pub use scheduler::DiskScheduler;

pub const PAGE_SIZE: usize = 4096;

/// DiskRequest uses page buffer seperately so
/// its same data but a cheap cope which we can mutate as well
pub type PageBuffer = Arc<RwLock<Box<[u8; PAGE_SIZE]>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestKind {
    Read,
    Write,
}

pub struct DiskRequest {
    pub kind: RequestKind,
    pub page_id: usize,
    pub data: PageBuffer,
    /// Completion signal only Carries status, never page data.
    pub done: SyncSender<io::Result<()>>,
}
