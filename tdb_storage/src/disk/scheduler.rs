use std::{
    io,
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
    thread::{self, JoinHandle},
};

use crate::disk::{DiskManager, DiskRequest};

// [Producer(BufferPool) tx -> rx Worker]
// [Worker(rx) -> performs IO -> done signal -> BufferPool]
pub struct DiskScheduler {
    tx: Option<Sender<DiskRequest>>,
    join: Option<JoinHandle<()>>,
}

impl DiskScheduler {
    pub fn new(disk_manager: DiskManager) -> Self {
        let (tx, rx) = mpsc::channel::<DiskRequest>();
        let disk_manager = Arc::new(disk_manager);

        let join = thread::spawn(move || {
            while let Ok(request) = rx.recv() {
                let done = request.done.clone();
                let result = disk_manager.perform(request);
                let _ = done.send(result);
            }
        });

        Self {
            tx: Some(tx),
            join: Some(join),
        }
    }

    pub fn schedule(&self, request: DiskRequest) -> io::Result<()> {
        let tx = self
            .tx
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "disk scheduler stopped"))?;

        tx.send(request)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "disk scheduler stopped"))
    }
}

impl Drop for DiskScheduler {
    fn drop(&mut self) {
        drop(self.tx.take());
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
