use core::mem;
use core::ptr::NonNull;

use crate::epoch::Epoch;
use crate::retired::Retired;

use arrayvec::{ArrayVec, IntoIter};

////////////////////////////////////////////////////////////////////////////////////////////////////
// EpochBagQueues
////////////////////////////////////////////////////////////////////////////////////////////////////

const BAG_QUEUE_COUNT: usize = 3;

/// A set of [`BagQueue`]s which store retired records and are perpetually
/// rotated and their contents reclaimed.
#[derive(Debug)]
pub(crate) struct EpochBagQueues {
    queues: [BagQueue; BAG_QUEUE_COUNT],
    curr_idx: usize,
}

impl EpochBagQueues {
    /// Creates a new empty set of [`BagQueue`]s.
    #[inline]
    pub fn new() -> Self {
        Self { queues: [BagQueue::new(), BagQueue::new(), BagQueue::new()], curr_idx: 0 }
    }

    /// Converts the three bag queues into **up to** three non-empty
    /// [`SealedQueues`].
    #[inline]
    pub fn into_sealed(
        self,
        current_epoch: Epoch,
    ) -> Option<(NonNull<SealedQueue>, NonNull<SealedQueue>)> {
        let mut iter: IntoIter<[BagQueue; BAG_QUEUE_COUNT]> = self.into_sorted().into_iter();
        iter.enumerate().filter_map(|(idx, queue)| queue.into_sealed(current_epoch - idx)).fold(
            None,
            |acc, tail| match acc {
                Some((head, mut prev_tail)) => {
                    unsafe { prev_tail.as_mut().next = Some(tail) };
                    Some((head, tail))
                }
                None => Some((tail, tail)),
            },
        )
    }

    /// Retires the given `record` in the current [`BagQueue`].
    #[inline]
    pub fn retire_record(&mut self, record: Retired, bag_pool: &mut BagPool) {
        let curr = &mut self.queues[self.curr_idx];
        // the head bag is guaranteed to never be full
        unsafe { curr.head.retired_records.push_unchecked(record) };
        if curr.head.retired_records.is_full() {
            let mut old_head = bag_pool.allocate_bag();
            mem::swap(&mut curr.head, &mut old_head);
            curr.head.next = Some(old_head);
        }
    }

    /// Retires the given `record` in the current [`BagQueue`] as the final
    /// record of an exiting thread.
    ///
    /// # Safety
    ///
    /// After calling this method, no further calls to `retire_record` or
    /// `retire_final_record` must be made.
    #[inline]
    pub unsafe fn retire_final_record(&mut self, record: Retired) {
        let curr = &mut self.queues[self.curr_idx];
        curr.head.retired_records.push_unchecked(record);
    }

    /// Advances the current epoch bag and reclaims all records in the oldest
    /// bag.
    ///
    /// # Safety
    ///
    /// It must ensured that two full epochs have actually passed since the
    /// records in the oldest bag have been retired.
    #[inline]
    pub unsafe fn rotate_and_reclaim(&mut self, bag_pool: &mut BagPool) {
        self.curr_idx = (self.curr_idx + 1) % BAG_QUEUE_COUNT;
        self.queues[self.curr_idx].reclaim_full_bags(bag_pool);
    }

    #[inline]
    fn into_sorted(self) -> ArrayVec<[BagQueue; BAG_QUEUE_COUNT]> {
        let [a, b, c] = self.queues;
        match self.curr_idx {
            0 => ArrayVec::from([a, c, b]),
            1 => ArrayVec::from([b, a, c]),
            2 => ArrayVec::from([c, b, a]),
            _ => unreachable!(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// BagPool
////////////////////////////////////////////////////////////////////////////////////////////////////

const BAG_POOL_SIZE: usize = 16;

/// A pool for storing and recycling no longer used [`BagNode`]s of a thread.
#[derive(Debug)]
pub(crate) struct BagPool(ArrayVec<[Box<BagNode>; BAG_POOL_SIZE]>);

impl BagPool {
    #[inline]
    pub fn new() -> Self {
        Self(ArrayVec::default())
    }

    #[inline]
    fn allocate_bag(&mut self) -> Box<BagNode> {
        self.0.pop().unwrap_or_else(|| BagNode::boxed())
    }

    #[inline]
    fn recycle_bag(&mut self, bag: Box<BagNode>) {
        debug_assert_eq!(bag.retired_records.len(), 0);
        if let Err(cap) = self.0.try_push(bag) {
            mem::drop(cap.element());
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// SealedQueue
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub(crate) struct SealedSubList {
    head: NonNull<SealedQueue>,
    tail: NonNull<SealedQueue>,
}

impl SealedSubList {
    #[inline]
    fn new(head: NonNull<SealedQueue>) -> Self {
        Self { head, tail: head }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// SealedQueue
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub(crate) struct SealedQueue {
    next: Option<NonNull<SealedQueue>>,
    seal: Epoch,
    queue: BagQueue,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// BagQueue
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A LIFO queue of [`RetiredBag`]s.
///
/// All nodes except the first one are guaranteed to be full and the first one
/// is guaranteed to always have enough space for at least one additional
/// record.
#[derive(Debug)]
struct BagQueue {
    head: Box<BagNode>,
}

impl BagQueue {
    #[inline]
    fn new() -> Self {
        Self { head: BagNode::boxed() }
    }

    /// Returns `true` if the head node is both empty and has no successor.
    #[inline]
    fn is_empty(&self) -> bool {
        self.head.retired_records.len() == 0 && self.head.next.is_none()
    }

    /// Consumes `self` and drops the queue it is empty, otherwise returning the
    /// non-empty queue wrapped in a boxed [`SealedQueue`].
    #[inline]
    fn into_sealed(self, epoch: Epoch) -> Option<NonNull<SealedQueue>> {
        if !self.is_empty() {
            Some(NonNull::from(Box::leak(Box::new(SealedQueue {
                next: None,
                seal: epoch,
                queue: self,
            }))))
        } else {
            None
        }
    }

    /// # Safety
    ///
    /// It must be ensured that the contents of the queue are at least two
    /// epochs old.
    #[inline]
    unsafe fn reclaim_full_bags(&mut self, bag_pool: &mut BagPool) {
        let mut node = self.head.next.take();
        while let Some(mut bag) = node {
            node = bag.next.take();
            for record in bag.retired_records.drain(..) {
                record.reclaim();
            }

            bag_pool.recycle_bag(bag);
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// BagNode
////////////////////////////////////////////////////////////////////////////////////////////////////

const DEFAULT_BAG_SIZE: usize = 256;

#[derive(Debug)]
struct BagNode {
    next: Option<Box<BagNode>>,
    retired_records: ArrayVec<[Retired; DEFAULT_BAG_SIZE]>,
}

impl BagNode {
    #[inline]
    fn boxed() -> Box<Self> {
        Box::new(Self { next: None, retired_records: ArrayVec::default() })
    }
}
