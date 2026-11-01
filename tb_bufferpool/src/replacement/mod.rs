mod arc;
mod clock;
mod lru;

pub use arc::ArcReplacer;
pub use clock::ClockReplacer;
pub use lru::LRUReplacer;

/// Replacement policies that only know about frame slots.
///
/// They answer "which slot is coldest", which is all a policy needs when it
/// does not care what is inside the frame. ARC does not implement this trait:
/// its ghost lists are a directory of pages, so it needs page ids, not slots.
pub trait Replacer {
    /// Picks a victim and unpins it from the marker and returns
    /// its frame id, if it returns None then everything is pinned.  
    fn evict(&mut self) -> Option<usize>;

    fn pin(&mut self, frame_id: usize);

    fn unpin(&mut self, frame_id: usize);

    /// number of frames that are evictable  
    fn size(&self) -> usize;
}
