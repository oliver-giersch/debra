use core::ptr::NonNull;

use arrayvec::ArrayVec;

use crate::epoch::{Epoch, ThreadState};
use crate::list::Node;
use crate::retired::{BagQueue, Retired, SealedQueue};

const BAG_COUNT: usize = 3;

/// An array with **up to** three [`SealedQueue`]s.
pub(crate) type SealedEpochBags = ArrayVec<[Box<SealedQueue>; BAG_COUNT]>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// EpochBags
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub(super) struct EpochBags {
    curr_idx: usize,
    queues: [BagQueue; BAG_COUNT],
}

impl EpochBags {
    /// Creates a new empty set of [`EpochBags`].
    #[inline]
    pub fn new() -> Self {
        Self { curr_idx: 0, queues: [BagQueue::new(), BagQueue::new(), BagQueue::new()] }
    }

    /// Converts the three bag queues into **up to** three non-empty
    /// [`SealedQueues`].
    #[inline]
    pub fn into_sealed(self, current_epoch: Epoch) -> SealedEpochBags {
        self.into_sorted()
            .into_iter()
            .enumerate()
            .filter_map(|(idx, queue)| queue.non_empty().map(|queue| (idx, queue)))
            .map(|(idx, queue)| queue.seal(current_epoch - idx))
            .collect()
    }

    /// Retires the given `record` in the current [`BagQueue`].
    #[inline]
    pub fn retire_record(&mut self, record: Retired) {
        let curr = &mut self.queues[self.curr_idx];
        curr.retire_record(record);
    }

    /// Retires the given [`ThreadState`] node pointer.
    #[inline]
    pub unsafe fn retire_thread_state(&mut self, state: NonNull<Node<ThreadState>>) {
        let curr = &mut self.queues[self.curr_idx];
        curr.retire_thread_state(state);
    }

    /// Advances the current epoch bag and reclaims all records in the oldest
    /// bag.
    ///
    /// # Safety
    ///
    /// It must ensured that two full epochs have actually passed since the
    /// records in the oldest bag have been retired.
    #[inline]
    pub unsafe fn rotate_and_reclaim(&mut self) {
        self.curr_idx = (self.curr_idx + 1) % BAG_COUNT;
        self.queues[self.curr_idx].reclaim_full_bags();
    }

    #[inline]
    fn into_sorted(self) -> ArrayVec<[BagQueue; BAG_COUNT]> {
        let [a, b, c] = self.queues;
        match self.curr_idx {
            0 => ArrayVec::from([a, c, b]),
            1 => ArrayVec::from([b, a, c]),
            2 => ArrayVec::from([c, b, a]),
            _ => unreachable!(),
        }
    }
}
