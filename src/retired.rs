//! Type-erased caching of retired records

use core::fmt;
use core::mem::{self, ManuallyDrop};
use core::ptr::NonNull;

use arrayvec::ArrayVec;

use crate::epoch::Epoch;

////////////////////////////////////////////////////////////////////////////////////////////////////
// SealedQueue
////////////////////////////////////////////////////////////////////////////////////////////////////

pub(crate) struct SealedQueue {
    epoch: Epoch,
    queue: BagQueue,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// BagQueue
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A LIFO queue of [`RetiredBag`]s.
#[derive(Debug)]
pub(crate) struct BagQueue {
    head: Box<RetiredBag>,
}

impl BagQueue {
    #[inline]
    pub fn retire_record(&mut self, record: Retired) {
        // the head bag is guaranteed to never be full
        unsafe { self.head.retired.push_unchecked(record) };
        if self.head.retired.is_full() {
            self.swap_head();
        }
    }

    /// # Safety
    ///
    /// After calling this method, no further calls to [`retire_record`] must be
    /// made.
    #[inline]
    pub unsafe fn retire_thread_state(&mut self, state: NonNull<ThreadEpoch>) {
        self.head.retired.push_unchecked(Retired::new_unchecked(record));
    }

    #[inline]
    pub unsafe fn reclaim_full_bags(&mut self) {
        let mut node = self.head.next.take();
        while let Some(mut bag) = node {
            node = bag.next.take();
            for mut record in bag.retired.drain(..) {
                record.reclaim();
            }
        }
    }

    #[inline]
    fn swap_head(&mut self) {
        let mut old_head = Box::new(RetiredBag::new());
        mem::swap(&mut self.head, &mut old_head);
        self.head.next = Some(old_head);
    }
}

// TODO: impl Drop (clear head + call reclaim_full_bags)
//       when? other thread drops BagQueue as part of SealedQueue

////////////////////////////////////////////////////////////////////////////////////////////////////
// RetiredBag
////////////////////////////////////////////////////////////////////////////////////////////////////

const DEFAULT_BAG_SIZE: usize = 256;

#[derive(Debug)]
pub(crate) struct RetiredBag {
    next: Option<Box<RetiredBag>>,
    retired: ArrayVec<[Retired; DEFAULT_BAG_SIZE]>,
}

impl RetiredBag {
    #[inline]
    fn new() -> Self {
        Self { next: None, retired: ArrayVec::default() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retired
////////////////////////////////////////////////////////////////////////////////////////////////////

type Record<T> = reclaim::Record<T, crate::Debra>;

pub(crate) struct Retired(ManuallyDrop<Box<dyn Any + 'static>>);

impl Retired {
    #[inline]
    pub unsafe fn new_unchecked<'a, T: 'a>(record: NonNull<T>) -> Self {
        let any: NonNull<dyn Any + 'a> = record;
        let any: NonNull<dyn Any + 'static> = mem::transmute(any);

        Self(ManuallyDrop::new(Box::from_raw(any.as_ptr())))
    }

    #[inline]
    pub fn address(&self) -> usize {
        &*self.0 as *const _ as *const () as usize
    }

    #[inline]
    unsafe fn reclaim(&mut self) {
        ManuallyDrop::drop(&mut self.0);
    }
}

impl fmt::Debug for Retired {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Retired").field("address", &(self.address() as *const ())).finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Any (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

trait Any {}
impl<T> Any for T {}
