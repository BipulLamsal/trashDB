/*LRU approximate clock implementation*/

use super::Replacer;

struct Frame {
    /// Not a bit tho, a whole byte :(
    referenced: bool,
    pin: bool,
}

pub struct ClockReplacer {
    /// Maximum number of frames the clock can track
    capacity: usize,
    list: Vec<Frame>,
    hand: usize,
}

impl ClockReplacer {
    pub fn init(capacity: usize) -> Self {
        Self {
            capacity,
            list: (0..capacity)
                .map(|_| Frame {
                    referenced: false,
                    pin: true,
                })
                .collect(),
            hand: 0,
        }
    }
}

impl Replacer for ClockReplacer {
    fn pin(&mut self, frame_id: usize) {
        if frame_id >= self.capacity {
            return;
        }
        self.list[frame_id].pin = true;
    }

    fn evict(&mut self) -> Option<usize> {
        let n = self.capacity;
        if n == 0 {
            return None;
        }

        // a single pass is used to check if
        // its referenced or not but the second
        // pass is when we sitll dont find out
        //  and this time we ignore the referenced
        //  bit
        for _ in 0..2 * n {
            let id = self.hand;
            self.hand = (self.hand + 1) % n;

            let frame = &mut self.list[id];

            if frame.pin {
                continue;
            }

            if frame.referenced {
                frame.referenced = false;
            } else {
                frame.pin = true;
                return Some(id);
            }
        }

        None
    }

    fn unpin(&mut self, frame_id: usize) {
        if frame_id >= self.capacity {
            return;
        }

        self.list[frame_id].pin = false;
        self.list[frame_id].referenced = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_replacement() {
        let mut replacer = ClockReplacer::init(10);
        assert_eq!(replacer.evict(), None);
    }

    #[test]
    fn test_unpin_sets_reference_bit() {
        let mut replacer = ClockReplacer::init(10);
        assert!(replacer.list[0].pin);

        replacer.unpin(0);

        assert!(!replacer.list[0].pin);
        assert!(replacer.list[0].referenced);
    }

    #[test]
    fn test_evict_gives_second_chance() {
        let mut replacer = ClockReplacer::init(10);
        replacer.unpin(0);

        // first pass clears the reference bit and second pass evicts it
        assert_eq!(replacer.evict(), Some(0));
        assert!(replacer.list[0].pin);
    }

    #[test]
    fn test_eviction_order() {
        let mut replacer = ClockReplacer::init(10);
        replacer.unpin(0);
        replacer.unpin(1);
        replacer.unpin(2);

        assert_eq!(replacer.evict(), Some(0));
        assert_eq!(replacer.evict(), Some(1));
        assert_eq!(replacer.evict(), Some(2));
        assert_eq!(replacer.evict(), None);
    }

    #[test]
    fn test_out_of_range_frame() {
        let mut replacer = ClockReplacer::init(10);

        replacer.unpin(99);
        replacer.pin(99);

        assert_eq!(replacer.evict(), None);
    }
}
