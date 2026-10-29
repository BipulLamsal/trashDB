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
        let frame = &mut self.list[frame_id];
        if frame.pin {
            return;
        }
        frame.pin = true;
        let (prev, next) = (frame.prev, frame.next);

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
        if !self.list[frame_id].pin {
            return;
        }
        self.push(frame_id);
    }
}
