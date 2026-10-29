mod clock;
mod lru;

pub use clock::ClockReplacer;
pub use lru::LRUReplacer;

pub trait Replacer {
    /// Picks a victim and unpins it from the marker and returns
    /// its frame id, if it returns None then everything is pinned.  
    fn evict(&mut self) -> Option<usize>;

    fn pin(&mut self, frame_id: usize);

    fn unpin(&mut self, frame_id: usize);
}
