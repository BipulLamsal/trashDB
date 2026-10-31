/*
 * The LRU replacment policy
 * */

use super::Replacer;

/// A rust idomatic version of creating a
/// doubly linked list, frame here means a
/// Node,
#[derive(Debug, PartialEq)]
struct Frame {
    prev: Option<usize>,
    next: Option<usize>,
    pin: bool,
}

pub struct LRUReplacer {
    /// Maximum number of pages LRU can store
    capacity: usize,
    /// Track of Frames in use  
    list: Vec<Frame>,
    /// Recently used   
    head: Option<usize>,
    /// Least Recently used
    tail: Option<usize>,
    /// Number of unpinned frames currently in the list
    size: usize,
}

impl LRUReplacer {
    pub fn init(capacity: usize) -> Self {
        Self {
            capacity,
            list: (0..capacity)
                .map(|_| Frame {
                    prev: None,
                    next: None,
                    pin: true,
                })
                .collect(),
            head: None,
            tail: None,
            size: 0,
        }
    }

    /// Pushes the value and updates the head
    /// this function doesnot check if the frame_id
    /// is pinned or unpinned, it just pushes the frame
    /// caller before pushing should make
    /// It is guarantted that push will update the list.
    fn push(&mut self, frame_id: usize) {
        if let Some(head_id) = self.head {
            self.list[head_id].prev = Some(frame_id);
            self.list[frame_id] = Frame {
                prev: None,
                next: Some(head_id),
                pin: false,
            };
            self.head = Some(frame_id);
        } else {
            self.list[frame_id] = Frame {
                prev: None,
                next: None,
                pin: false,
            };
            self.head = Some(frame_id);
            self.tail = Some(frame_id);
        }
        self.size += 1;
    }
}

impl Replacer for LRUReplacer {
    /// Returns the victims frame id for eviction, if
    /// nothing to evict it returns None that also means
    /// everything is pinned.
    fn evict(&mut self) -> Option<usize> {
        let tail_id = self.tail?;
        // remove it from the list
        self.pin(tail_id);
        return Some(tail_id);
    }

    /// Pins removes the frame from the list so that
    /// means we need to remove it from the linkedlist
    /// as well, the previous and after elements of
    /// whatever we are pinning should be pointed to
    /// each other.

    fn pin(&mut self, frame_id: usize) {
        if frame_id >= self.capacity {
            return;
        }

        let frame = &mut self.list[frame_id];
        if frame.pin {
            return;
        }
        frame.pin = true;
        let (prev, next) = (frame.prev, frame.next);
        self.size -= 1;

        // make sure we remove form the linked list
        match (prev, next) {
            (Some(prev), Some(next)) => {
                self.list[prev].next = Some(next);
                self.list[next].prev = Some(prev);
            }
            (None, Some(next)) => {
                self.list[next].prev = None;
                self.head = Some(next);
            }
            (Some(prev), None) => {
                self.list[prev].next = None;
                self.tail = Some(prev);
            }
            _ => {
                // only if the list
                // has one only one element
                self.head = None;
                self.tail = None;
            }
        };
    }

    /// keep track of the frame_id in
    /// replacment cycle
    fn unpin(&mut self, frame_id: usize) {
        if frame_id >= self.capacity {
            return;
        }

        if !self.list[frame_id].pin {
            return;
        }

        self.push(frame_id);
    }

    fn size(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unpin() {
        // buffer pool capacity is same as the replacer
        // capacity and frame id is unique
        let mut replacer = LRUReplacer::init(10);
        assert!(replacer.list[0].pin);
        replacer.unpin(0);
        assert!(!replacer.list[0].pin);
        let victim = replacer.evict();

        assert!(victim.unwrap() == 0);
    }

    #[test]
    fn test_no_replacement() {
        let mut replacer = LRUReplacer::init(10);
        assert!(replacer.evict() == None);
    }

    #[test]
    fn test_eviction() {
        let mut replacer = LRUReplacer::init(10);
        replacer.unpin(0);
        replacer.unpin(1);
        replacer.unpin(2);

        assert!(replacer.tail.unwrap() == 0);
    }

    #[test]
    fn test_size_tracks_evictable_frames() {
        let mut replacer = LRUReplacer::init(10);
        assert_eq!(replacer.size(), 0);

        replacer.unpin(0);
        replacer.unpin(1);
        assert_eq!(replacer.size(), 2);

        // pinning removes the frame from the list
        replacer.pin(0);
        assert_eq!(replacer.size(), 1);

        // evicting pins the victim as well
        assert_eq!(replacer.evict(), Some(1));
        assert_eq!(replacer.size(), 0);

        // unpinning twice must not double count
        replacer.unpin(0);
        replacer.unpin(0);
        assert_eq!(replacer.size(), 1);
    }

    #[test]
    fn test_pin_in_between_frames() {
        let mut replacer = LRUReplacer::init(10);
        replacer.unpin(0);
        replacer.unpin(1);
        replacer.unpin(2);
        replacer.pin(1);

        assert_eq!(replacer.list[2].next, Some(0));
        assert_eq!(replacer.list[0].next, replacer.list[2].prev);
        assert_eq!(replacer.list[0].prev, Some(2));
    }
}
