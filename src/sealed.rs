//! Abandoned epoch queues sealed with epoch information.

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use core::ptr::NonNull;

use debra_common::arrayvec::ArrayVec;
use debra_common::epoch::Epoch;

type BagNode = debra_common::bag::BagNode<crate::Debra>;
type BagQueue = debra_common::bag::BagQueue<crate::Debra>;
type EpochBagQueues = debra_common::bag::EpochBagQueues<crate::Debra>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// SealedList
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A linked sub-list consisting of up to three `BagQueue`s sealed with the
/// epoch during which its contents were retired.
#[derive(Debug)]
pub(crate) struct SealedList(NonNull<Sealed>, NonNull<Sealed>);

/***** impl inherent ******************************************************************************/

impl SealedList {
    #[inline]
    pub fn from_bags(bags: EpochBagQueues, current_epoch: Epoch) -> Option<Self> {
        let iter = ArrayVec::from(bags.into_sorted()).into_iter();
        iter.enumerate()
            .filter_map(|(idx, queue)| Sealed::from_queue(queue, current_epoch - idx))
            .fold(None, |acc, tail| match acc {
                Some(SealedList(head, mut prev_tail)) => {
                    unsafe { prev_tail.as_mut().next = Some(tail) };
                    Some(SealedList(head, tail))
                }
                None => Some(SealedList(tail, tail)),
            })
    }

    #[inline]
    pub fn into_inner(self) -> (NonNull<Sealed>, NonNull<Sealed>) {
        (self.0, self.1)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Sealed
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub(crate) struct Sealed {
    pub(crate) next: Option<NonNull<Sealed>>,
    pub(crate) seal: Epoch,
    queue: Box<BagNode>,
}

/***** impl inherent ******************************************************************************/

impl Sealed {
    #[inline]
    fn from_queue(queue: BagQueue, epoch: Epoch) -> Option<NonNull<Self>> {
        queue.into_non_empty().map(|queue| {
            NonNull::from(Box::leak(Box::new(Self { next: None, seal: epoch, queue })))
        })
    }
}

/***** impl Drop **********************************************************************************/

impl Drop for Sealed {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.queue.reclaim_all() };
    }
}
