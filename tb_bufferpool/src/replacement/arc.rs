/* Adaptive Replacement Policy */

/* Idea from the paper
*
*  - maintain two LRU list
*  - L1: only once recently (recency)
*  - L2: atleaset twice recently (frequency)
*  -  The items that have been seen twice withtin a short interval of time
*      have low iter arrivial rate hence are called high frequency.
*
*  - The cache can be tuned with some parameter(p) such that
*  the cache has both of the list value p and c-p where c is total capacity.
*  - FRC is fixed meaning it cannot change the value of p.
*  - ARC here means p walks randomly towards more top performing
*  L1 or L2 at least what papper says..
*  - offline tuning: pick the paramter before runnign by observation of previous
*  - online tuning: figure things out while running
*  - ARC has massive advantage than LRU with only 0.75% space overhead
*  - For a long running sequntial read or scan ARC can have major advantage
*  as they dont dont compeletly ignore the hot pages or temporal locality page.
*  - Paper also disccus about hit ratio of 20% in certain workload setting, see paper abstract.
*  - Skipped section 3, i mean i read the summary; it just shows why its better. I will read
*  sometime later maybe
*
*  - Class of Replacement policies
*  - c = capacity, pi(c) meaans number of pages managed by the policy
*  - DBL(2c), that remebers twice the number of pages present in the cache
*  - It has one cache with 2 directory list L1, L2 where a page exist in L1 if its exactly used one time.
*  - If its in L2 then it has been requested more than one time since the last time
*  - the policy is : replace LRU page in L1, if L1 contains exactly c pages; otherwise
*  replace the LRU page in the L2.
*  - Attempts to equalize the size of the two lists. 0 =< L1 =< c; 0 <= L1+L2 <= 2c; 0<=L2<=2c
*  - Now the L1 and L2 can also be divided into top and bottom portion T and B such that
*  - Algorithm:
*
*
* Just think of L1 and L2 are just part of a singel list with differnt index
* so idea is fairly simple just 2 list
*
* [[[LRU] L1 [MRU]][[MRU] L2 [LRU]]] - the whole thign is 2c
*
*  input: x1,x2,x3,...xt
*  init: l1=0,l2=0 L1=NULL, L2=NULL
*
*  if (t>=0 & xt as page_id) {
*  match page_id {
*       page_id exist in L1 or L2 => {
*           // Cache HIT yipeeiee
*           make_mru(L2, page_id);
*       },
*       page_id not in both {
*           // Cache Miss
*           if (l1.size() == capacity) {
*               evitct(L1);
*               make_mru(l1, page_id);
*           } else if (
*               l1.size() < capacity
*           ) {
*               // but its still full then
*               // make a room for new page
*               if (L1+L2 == 2c) {
*                   evitct(L2);
*               }
*               make_mru(l1);
*
*           }
*       }
*  }
*  }
*
*
* okay now what if we further divide this to [B1 T1] where T1 here means MRU and B1 means LRU, [T2 B2]
* so Algorithm policy is such as:
* - if t1 > p then for sure the paramter exccedds whats required so evitct(T1)
* - if t1 < p then evitct(T2)
* - if (t1==p) this is more interesting because now we need to look into ghost cache
*   - if B1 has the frame then replace in T2
*   - if B2 has the frame then replace in T1
*
*   but since this is fixed replacement policy we dont tpyically change the value of p in this case

* p = target size for T1
* Adptation really depends on the paramter p
* when p is close to zero the Algorithm is favoring L2
*
* whats the learning?
*
* if there is hit in B1 then we need to increase the size of t1,
* this means ARC focus on recent page list more
* if there is hit in b2 then we need to increase size of t2,
* this measn ARC focus on frequency
*
* the delta is 1 if B1 >= B2  or viceversa else
* delta is b2/b1 or b1/b2  (num>den) so that in case of hit in b1 lets say if b1 is actually less
* then we give more then 1 change.
*
*
* invest in the list that is performing the best, this is what the random walk
* the papaer talks previously
*
* */

use std::{
    cmp,
    collections::{HashMap, VecDeque},
};

use crate::replacement::{LRUReplacer, Replacer};

#[derive(Debug, Clone, Copy, PartialEq)]
enum AliveList {
    /// Seen exactly once recently
    MRU,
    /// Seen at least twice recently
    MFU,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GhostList {
    /// Ghost entry for a page evicted from T1
    MruGhost,
    /// Ghost entry for a page evicted from T2
    MfuGhost,
}

struct FrameStatus {
    page_id: usize,
    frame_id: usize,
    /// evictable can be used as pinned?
    evictable: bool,
    list: AliveList,
}

pub struct ArcReplacer {
    /// Evictable entries count
    size: usize,
    capacity: usize,
    target: usize,
    mru_list: LRUReplacer,
    mfu_list: LRUReplacer,

    /// size that also included pin or unpinned
    mru_size: usize,
    /// size that also included pin or unpinned
    mfu_size: usize,

    /// ghosts uses page id
    mru_ghost_list: VecDeque<usize>,
    mfu_ghost_list: VecDeque<usize>,

    /// records entries in mru and mfu
    /// this uses frame_id to guarantee no duplicate records
    /// for the same frame when they are alive
    alive_map: HashMap<usize, FrameStatus>,

    /// record entries in mru ghost and mfu ghost
    ghost_map: HashMap<usize, GhostList>,
}

impl ArcReplacer {
    pub fn init(capacity: usize) -> Self {
        Self {
            size: 0,
            capacity,
            target: 0,
            mru_list: LRUReplacer::init(capacity),
            mfu_list: LRUReplacer::init(capacity),
            mru_size: 0,
            mfu_size: 0,
            mru_ghost_list: VecDeque::new(),
            mfu_ghost_list: VecDeque::new(),
            alive_map: HashMap::new(),
            ghost_map: HashMap::new(),
        }
    }

    pub fn evict(&mut self) -> Option<usize> {
        let prefer_mru = self.mru_size > self.target;

        let victim = if prefer_mru {
            self.mru_list.evict().or_else(|| self.mfu_list.evict())
        } else {
            self.mfu_list.evict().or_else(|| self.mru_list.evict())
        }?;

        let frame = self
            .alive_map
            .remove(&victim)
            .expect("map should contain frame");

        self.size -= 1;

        match frame.list {
            AliveList::MRU => {
                self.mru_size -= 1;
                self.mru_ghost_list.push_front(frame.page_id);
                self.ghost_map.insert(frame.page_id, GhostList::MruGhost);
            }
            AliveList::MFU => {
                self.mfu_size -= 1;
                self.mfu_ghost_list.push_front(frame.page_id);
                self.ghost_map.insert(frame.page_id, GhostList::MfuGhost);
            }
        }

        Some(frame.frame_id)
    }
    /// record access to the frame as told by the bufferpool
    /// to record somethign it has four cases
    /// record the access after evict or after pulling the page  
    pub fn record_access(&mut self, frame_id: usize, page_id: usize) {
        // frame is alive check t1 and t2

        if let Some(alive_map) = self.alive_map.get_mut(&frame_id) {
            match alive_map.list {
                AliveList::MRU => {
                    // case 1 when its in mru
                    // promote to t2 and mark mfu
                    // Invariant the repalcer only has the frame if its evictable
                    if alive_map.evictable {
                        self.mru_list.pin(frame_id);
                        self.mfu_list.unpin(frame_id);
                    }
                    self.mru_size -= 1;
                    self.mfu_size += 1;
                    alive_map.list = AliveList::MFU;
                }
                AliveList::MFU => {
                    // case 2
                    // already frequent just move it
                    if alive_map.evictable {
                        self.mfu_list.pin(frame_id);
                        self.mfu_list.unpin(frame_id);
                    }
                }
            }
        } else if let Some(ghost_list) = self.ghost_map.get(&page_id).copied() {
            // case 3
            let b1 = self.mru_ghost_list.len();
            let b2 = self.mfu_ghost_list.len();

            match ghost_list {
                GhostList::MruGhost => {
                    let delta = if b1 >= b2 { 1 } else { b2 / b1 };
                    self.target = cmp::min(self.capacity, self.target.saturating_add(delta));
                    // remove the page_id: NEEDS OPTIMIZATION : O(N)
                    self.mru_ghost_list.retain(|&p| p != page_id);
                }
                GhostList::MfuGhost => {
                    let delta = if b2 >= b1 { 1 } else { b1 / b2 };
                    self.target = self.target.saturating_sub(delta);
                    self.mfu_ghost_list.retain(|&p| p != page_id);
                }
            }

            self.ghost_map.remove(&page_id);
            debug_assert!(self.mru_size + self.mfu_size < self.capacity);
            self.alive_map.insert(
                frame_id,
                FrameStatus {
                    page_id,
                    frame_id,
                    evictable: false,
                    list: AliveList::MFU,
                },
            );
            self.mfu_size += 1;
        } else {
            // most interesting
            // frame is no where case 4 which is the last case
            // since we accessed we need to update the mru lists
            // but before that we need to check if things are full?

            let t1_size = self.mru_size + self.mru_ghost_list.len();
            debug_assert!(t1_size <= self.capacity);

            // if recently used is full then make a room from mru ghost
            if t1_size == self.capacity {
                if let Some(old) = self.mru_ghost_list.pop_back() {
                    self.ghost_map.remove(&old);
                }
            // if whole directory lis is full make a room from mfu ghost
            } else if t1_size + self.mfu_size + self.mfu_ghost_list.len() == 2 * self.capacity {
                if let Some(old) = self.mfu_ghost_list.pop_back() {
                    self.ghost_map.remove(&old);
                }
            }

            self.alive_map.insert(
                frame_id,
                FrameStatus {
                    page_id,
                    frame_id,
                    evictable: false,
                    list: AliveList::MRU,
                },
            );
            self.mru_size += 1;
        }
    }

    pub fn set_evictable(&mut self, frame_id: usize, state: bool) {
        let Some(frame) = self.alive_map.get_mut(&frame_id) else {
            // none
            return;
        };

        if frame.evictable == state {
            return;
        }

        frame.evictable = state;

        match (frame.list, state) {
            (AliveList::MRU, true) => self.mru_list.unpin(frame_id),
            (AliveList::MRU, false) => self.mru_list.pin(frame_id),
            (AliveList::MFU, true) => self.mfu_list.unpin(frame_id),
            (AliveList::MFU, false) => self.mfu_list.pin(frame_id),
        }

        if state {
            self.size += 1;
        } else {
            self.size -= 1;
        }
    }

    /// remove the frame from the replacer entierly
    pub fn remove(&mut self, frame_id: usize) {
        let Some(frame) = self.alive_map.get(&frame_id) else {
            return; // unknown frame   
        };

        assert!(frame.evictable, "cannot remove a unevictable frame");

        let list = frame.list;
        match list {
            AliveList::MRU => {
                self.mru_list.pin(frame_id);
                self.mru_size -= 1;
            }
            AliveList::MFU => {
                self.mfu_list.pin(frame_id);
                self.mfu_size -= 1;
            }
        }

        self.alive_map.remove(&frame_id);
        self.size -= 1;
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_page_starts_in_mru_and_pinned() {
        let mut replacer = ArcReplacer::init(4);

        replacer.record_access(0, 10);

        assert_eq!(replacer.mru_size, 1);
        assert_eq!(replacer.mfu_size, 0);
        assert_eq!(replacer.size(), 0);
        assert_eq!(replacer.mru_list.size(), 0);
        assert!(!replacer.alive_map[&0].evictable);
        assert_eq!(replacer.alive_map[&0].list, AliveList::MRU);
        assert_eq!(replacer.alive_map[&0].page_id, 10);
    }

    #[test]
    fn mru_record_already_exist() {
        let mut replacer = ArcReplacer::init(4);
        replacer.record_access(0, 10);

        // unpin it
        replacer.set_evictable(0, true);
        // case 1
        replacer.record_access(0, 10);

        assert_eq!(replacer.mru_size, 0);
        assert_eq!(replacer.mfu_size, 1);
        assert_eq!(replacer.size(), 1);
        assert_eq!(replacer.mru_list.size(), 0);
        assert_eq!(replacer.mfu_list.size(), 1);
        assert_eq!(replacer.alive_map[&0].list, AliveList::MFU);
    }

    #[test]
    fn pinned_mru_hit_still_promotes_to_mfu() {
        let mut replacer = ArcReplacer::init(4);
        replacer.record_access(0, 10);
        replacer.record_access(0, 10);

        assert_eq!(replacer.mru_size, 0);
        // this includes evictable as false i.e  pinned
        assert_eq!(replacer.mfu_size, 1);
        assert_eq!(replacer.mfu_list.size(), 0);
        assert_eq!(replacer.size(), 0);

        replacer.set_evictable(0, true);

        assert_eq!(replacer.mfu_list.size(), 1);
        assert_eq!(replacer.size(), 1);
    }

    #[test]
    fn mfu_double_hit_updates() {
        let mut replacer = ArcReplacer::init(4);
        for (frame_id, page_id) in [(0, 10), (1, 11)] {
            replacer.record_access(frame_id, page_id);
            replacer.set_evictable(frame_id, true);
            replacer.record_access(frame_id, page_id);
        }

        replacer.record_access(0, 10);

        assert_eq!(replacer.evict(), Some(1));
        assert_eq!(replacer.mfu_ghost_list.front(), Some(&11));
        assert_eq!(replacer.ghost_map.get(&11), Some(&GhostList::MfuGhost));
    }

    #[test]
    fn evict_mru_moves_page_to_mru_ghost() {
        let mut replacer = ArcReplacer::init(4);
        replacer.record_access(0, 10);
        replacer.set_evictable(0, true);
        replacer.record_access(1, 11);
        replacer.set_evictable(1, true);

        assert_eq!(replacer.evict(), Some(0));

        assert_eq!(replacer.mru_size, 1);
        assert_eq!(replacer.size(), 1);
        assert!(!replacer.alive_map.contains_key(&0));
        assert_eq!(replacer.mru_ghost_list.front(), Some(&10));
        assert_eq!(replacer.ghost_map.get(&10), Some(&GhostList::MruGhost));
    }

    #[test]
    fn evict_mfu_moves_page_to_mfu_ghost() {
        let mut replacer = ArcReplacer::init(4);
        replacer.record_access(0, 10);
        replacer.set_evictable(0, true);
        replacer.record_access(0, 10); // promote to mfu_list

        assert_eq!(replacer.evict(), Some(0));

        assert_eq!(replacer.mfu_size, 0);
        assert_eq!(replacer.size(), 0);
        assert!(!replacer.alive_map.contains_key(&0));
        assert_eq!(replacer.mfu_ghost_list.front(), Some(&10));
        assert_eq!(replacer.ghost_map.get(&10), Some(&GhostList::MfuGhost));
    }

    #[test]
    fn ghost_hit_from_mru_ghost_promotes_and_raises_target() {
        let mut replacer = ArcReplacer::init(4);
        for (frame_id, page_id) in [(0, 10), (1, 11), (2, 12), (3, 13)] {
            replacer.record_access(frame_id, page_id);
            replacer.set_evictable(frame_id, true);
        }

        assert_eq!(replacer.evict(), Some(0));
        assert_eq!(replacer.target, 0);

        replacer.record_access(0, 10);

        assert_eq!(replacer.target, 1);
        assert_eq!(replacer.mru_size, 3);
        assert_eq!(replacer.mfu_size, 1);
        assert_eq!(replacer.alive_map[&0].list, AliveList::MFU);
        assert!(!replacer.alive_map[&0].evictable);
        assert!(replacer.mru_ghost_list.is_empty());
        assert!(!replacer.ghost_map.contains_key(&10));

        replacer.set_evictable(0, true);
        assert_eq!(replacer.size(), 4);
    }

    #[test]
    fn ghost_hit_from_mfu_ghost_promotes_and_lowers_target() {
        let mut replacer = ArcReplacer::init(4);
        replacer.record_access(0, 10);
        replacer.set_evictable(0, true);
        replacer.record_access(1, 11);
        replacer.set_evictable(1, true);

        assert_eq!(replacer.target, 0);
        // page 10 leaves mru and enters mru_ghost_list
        assert_eq!(replacer.evict(), Some(0));
        // reaccessing it from mru_ghost_list raises the target and puts it in mfu_list
        replacer.record_access(0, 10);
        assert_eq!(replacer.target, 1);
        replacer.set_evictable(0, true);

        // page 10 leaves mfu_list and enters mfu_ghost_list
        assert_eq!(replacer.evict(), Some(0));
        assert_eq!(replacer.mfu_ghost_list.front(), Some(&10));

        // reaccessing it from mfu_ghost_list lowers the target again
        replacer.record_access(0, 10);

        assert_eq!(replacer.target, 0);
        assert_eq!(replacer.alive_map[&0].list, AliveList::MFU);
        assert!(replacer.mfu_ghost_list.is_empty());
        assert!(!replacer.ghost_map.contains_key(&10));
    }

    #[test]
    fn set_evictable_tracks_size_and_lru() {
        let mut replacer = ArcReplacer::init(4);
        replacer.record_access(0, 10);

        assert_eq!(replacer.size(), 0);
        assert_eq!(replacer.mru_list.size(), 0);

        replacer.set_evictable(0, true);
        assert_eq!(replacer.size(), 1);
        assert_eq!(replacer.mru_list.size(), 1);

        // setting the same state twice must not double count
        replacer.set_evictable(0, true);
        assert_eq!(replacer.size(), 1);

        replacer.set_evictable(0, false);
        assert_eq!(replacer.size(), 0);
        assert_eq!(replacer.mru_list.size(), 0);

        replacer.set_evictable(0, false);
        assert_eq!(replacer.size(), 0);
    }

    #[test]
    fn remove_drops_evictable_frame() {
        let mut replacer = ArcReplacer::init(4);
        replacer.record_access(0, 10);
        replacer.set_evictable(0, true);
        replacer.record_access(1, 11);
        replacer.set_evictable(1, true);

        replacer.remove(0);

        assert_eq!(replacer.size(), 1);
        assert_eq!(replacer.mru_size, 1);
        assert_eq!(replacer.mru_list.size(), 1);
        assert!(!replacer.alive_map.contains_key(&0));
        assert!(replacer.alive_map.contains_key(&1));
    }

    #[test]
    #[should_panic(expected = "cannot remove a unevictable frame")]
    fn remove_pinned_frame_panics() {
        let mut replacer = ArcReplacer::init(4);
        replacer.record_access(0, 10);

        replacer.remove(0);
    }

    #[test]
    fn evict_falls_back_to_mfu_when_mru_is_pinned() {
        let mut replacer = ArcReplacer::init(4);

        // frame 0 is in mru but pinned, so it is not in mru_list
        replacer.record_access(0, 10);

        // frame 1 is evictable and lives in mfu_list
        replacer.record_access(1, 11);
        replacer.set_evictable(1, true);
        replacer.record_access(1, 11);

        assert_eq!(replacer.mru_size, 1);
        assert_eq!(replacer.mfu_size, 1);

        // mru_size > target, but mru_list has no evictable frame; fall back to mfu_list
        assert_eq!(replacer.evict(), Some(1));

        assert_eq!(replacer.mfu_size, 0);
        assert_eq!(replacer.mru_size, 1);
        assert_eq!(replacer.mfu_ghost_list.front(), Some(&11));
        assert_eq!(replacer.ghost_map.get(&11), Some(&GhostList::MfuGhost));
    }

    #[test]
    fn evict_returns_none_when_all_frames_are_pinned() {
        let mut replacer = ArcReplacer::init(4);
        replacer.record_access(0, 10);

        assert_eq!(replacer.evict(), None);

        assert_eq!(replacer.size(), 0);
        assert!(replacer.alive_map.contains_key(&0));
        assert!(replacer.mru_ghost_list.is_empty());
        assert!(replacer.mfu_ghost_list.is_empty());
        assert!(replacer.ghost_map.is_empty());
    }

    #[test]
    fn new_page_cleans_oldest_mru_ghost_when_t1_is_full() {
        let mut replacer = ArcReplacer::init(2);
        replacer.record_access(0, 10);
        replacer.set_evictable(0, true);
        replacer.record_access(1, 11);
        replacer.set_evictable(1, true);

        assert_eq!(replacer.evict(), Some(0));
        assert_eq!(replacer.mru_ghost_list.front(), Some(&10));

        // mru_size + mru_ghost_list.len() == capacity, so the oldest ghost is dropped
        replacer.record_access(0, 12);

        assert_eq!(replacer.mru_size, 2);
        assert_eq!(replacer.alive_map[&0].page_id, 12);
        assert!(replacer.mru_ghost_list.is_empty());
        assert!(replacer.ghost_map.is_empty());
        assert_eq!(replacer.size(), 1);
    }

    #[test]
    fn evict_respects_target() {
        let mut replacer = ArcReplacer::init(4);
        replacer.record_access(0, 10);
        replacer.set_evictable(0, true);
        replacer.record_access(1, 11);
        replacer.set_evictable(1, true);
        replacer.record_access(1, 11); // frame 1 is in mfu_list

        replacer.target = 1;

        // mru_size == target so mfu_list is evicted
        assert_eq!(replacer.evict(), Some(1));
        assert_eq!(replacer.mfu_ghost_list.front(), Some(&11));
    }

    #[test]
    fn evict_when_mru_list_is_full() {
        let mut replacer = ArcReplacer::init(4);
        for (frame_id, page_id) in [(0, 10), (1, 11), (2, 12), (3, 13)] {
            replacer.record_access(frame_id, page_id);
            replacer.set_evictable(frame_id, true);
        }

        assert_eq!(replacer.mru_size, 4);
        assert_eq!(replacer.mfu_size, 0);
        assert_eq!(replacer.mru_list.size(), 4);
        assert_eq!(replacer.mfu_list.size(), 0);

        assert_eq!(replacer.evict(), Some(0));

        assert_eq!(replacer.mru_size, 3);
        assert_eq!(replacer.size(), 3);
        assert_eq!(replacer.mru_list.size(), 3);
        assert_eq!(replacer.mru_ghost_list.front(), Some(&10));
        assert_eq!(replacer.ghost_map.get(&10), Some(&GhostList::MruGhost));

        // mru is full again after reusing frame 0, so the oldest mru ghost is dropped
        replacer.record_access(0, 14);

        assert_eq!(replacer.mru_size, 4);
        assert_eq!(replacer.mru_list.size(), 3);
        assert_eq!(replacer.alive_map[&0].page_id, 14);
        assert!(replacer.mru_ghost_list.is_empty());
        assert!(replacer.ghost_map.is_empty());
    }

    #[test]
    fn evict_when_mfu_list_is_full() {
        let mut replacer = ArcReplacer::init(4);
        for (frame_id, page_id) in [(0, 10), (1, 11), (2, 12), (3, 13)] {
            replacer.record_access(frame_id, page_id);
            replacer.set_evictable(frame_id, true);
            replacer.record_access(frame_id, page_id);
        }

        assert_eq!(replacer.mru_size, 0);
        assert_eq!(replacer.mfu_size, 4);
        assert_eq!(replacer.mru_list.size(), 0);
        assert_eq!(replacer.mfu_list.size(), 4);

        assert_eq!(replacer.evict(), Some(0));

        assert_eq!(replacer.mru_size, 0);
        assert_eq!(replacer.mfu_size, 3);
        assert_eq!(replacer.size(), 3);
        assert_eq!(replacer.mfu_list.size(), 3);
        assert_eq!(replacer.mfu_ghost_list.front(), Some(&10));
        assert_eq!(replacer.ghost_map.get(&10), Some(&GhostList::MfuGhost));

        replacer.record_access(0, 14);

        assert_eq!(replacer.mru_size, 1);
        assert_eq!(replacer.alive_map[&0].page_id, 14);
        assert_eq!(replacer.alive_map[&0].list, AliveList::MRU);
    }

    #[test]
    fn evict_when_mru_and_mfu_are_full() {
        let mut replacer = ArcReplacer::init(4);
        for (frame_id, page_id) in [(0, 10), (1, 11), (2, 12), (3, 13)] {
            replacer.record_access(frame_id, page_id);
            replacer.set_evictable(frame_id, true);
        }

        // promote frame 0 and frame 1, leaving frame 2 and frame 3 in mru_list
        replacer.record_access(0, 10);
        replacer.record_access(1, 11);

        assert_eq!(replacer.mru_size, 2);
        assert_eq!(replacer.mfu_size, 2);
        assert_eq!(replacer.mru_list.size(), 2);
        assert_eq!(replacer.mfu_list.size(), 2);
        assert_eq!(replacer.size(), 4);

        // mru_size > target, so the tail of mru_list is evicted first
        assert_eq!(replacer.evict(), Some(2));
        assert_eq!(replacer.mru_size, 1);
        assert_eq!(replacer.mfu_size, 2);
        assert_eq!(replacer.mru_ghost_list.front(), Some(&12));

        replacer.target = 1;

        // mru_size == target, so the tail of mfu_list is evicted next
        assert_eq!(replacer.evict(), Some(0));
        assert_eq!(replacer.mru_size, 1);
        assert_eq!(replacer.mfu_size, 1);
        assert_eq!(replacer.mfu_ghost_list.front(), Some(&10));
    }

    #[test]
    fn new_page_when_directory_is_full_drops_mfu_ghost() {
        let mut replacer = ArcReplacer::init(3);

        // fill all three frames with mru pages
        for (frame_id, page_id) in [(0, 10), (1, 11), (2, 12)] {
            replacer.record_access(frame_id, page_id);
            replacer.set_evictable(frame_id, true);
        }

        // promote all of them to mfu_list
        for (frame_id, page_id) in [(0, 10), (1, 11), (2, 12)] {
            replacer.record_access(frame_id, page_id);
        }

        // evict two mfu pages, so mfu_ghost_list has two entries
        assert_eq!(replacer.evict(), Some(0));
        assert_eq!(replacer.evict(), Some(1));

        assert_eq!(replacer.mru_size, 0);
        assert_eq!(replacer.mfu_size, 1);
        assert_eq!(replacer.mru_ghost_list.len(), 0);
        assert_eq!(replacer.mfu_ghost_list.len(), 2);

        // put page 13 in frame 0 and promote it to mfu_list
        replacer.record_access(0, 13);
        replacer.record_access(0, 13);

        assert_eq!(replacer.mru_size, 0);
        assert_eq!(replacer.mfu_size, 2);

        // put page 14 in frame 1, unpin it, then evict it to mru_ghost_list
        replacer.record_access(1, 14);
        replacer.set_evictable(1, true);
        assert_eq!(replacer.evict(), Some(1));

        assert_eq!(replacer.mru_size, 0);
        assert_eq!(replacer.mru_ghost_list.front(), Some(&14));

        replacer.record_access(1, 15);

        assert_eq!(replacer.mru_size, 1);
        assert_eq!(replacer.mfu_size, 2);
        assert_eq!(replacer.mru_ghost_list.len(), 1);
        assert_eq!(replacer.mfu_ghost_list.len(), 2);

        // evict frame 2 (the only evictable frame), then load page 16
        assert_eq!(replacer.evict(), Some(2));

        assert_eq!(replacer.mfu_size, 1);
        assert_eq!(replacer.mfu_ghost_list.len(), 3);

        replacer.record_access(2, 16);

        assert_eq!(replacer.mru_size, 2);
        assert_eq!(replacer.mfu_size, 1);
        assert_eq!(replacer.mru_ghost_list.front(), Some(&14));
        assert_eq!(replacer.mfu_ghost_list.front(), Some(&12));
        assert_eq!(replacer.mfu_ghost_list.back(), Some(&11));
        assert!(!replacer.ghost_map.contains_key(&10));
        assert_eq!(replacer.alive_map[&2].page_id, 16);
        assert_eq!(replacer.alive_map[&2].list, AliveList::MRU);
        assert_eq!(replacer.size(), 0);
    }
}
