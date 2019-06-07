//! Type-erased caching of retired records

use core::fmt;
use core::mem;
use core::ptr::NonNull;

use arrayvec::ArrayVec;

use crate::epoch::{Epoch, ThreadState};
use crate::list::Node;

////////////////////////////////////////////////////////////////////////////////////////////////////
// SealedQueue
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A [`BagQueue`] sealed with the [`Epoch`] in which its contained records were
/// retired.
#[derive(Debug)]
pub(crate) struct SealedQueue {
    seal: Epoch,
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
    /// Creates a new queue.
    #[inline]
    pub fn new() -> Self {
        Self { head: Box::new(RetiredBag::new()) }
    }

    /// Consumes `self` and drops the queue it is empty, otherwise returning the
    /// non-empty queue wrapped in a [`Some`].
    #[inline]
    pub fn non_empty(self) -> Option<BagQueue> {
        if self.head.retired.len() == 0 && self.head.next.is_none() {
            None
        } else {
            Some(self)
        }
    }

    /// Seals the [`BagQueue`] with the given [`Epoch`].
    #[inline]
    pub fn seal(self, seal: Epoch) -> Box<SealedQueue> {
        Box::new(SealedQueue { seal, queue: self })
    }

    /// Retires a record in the current first [`RetiredBag`].
    ///
    /// If the current bag becomes full due a call to this method, a new and
    /// empty one is allocated and inserted in its stead, the old one being
    /// pushed back.
    #[inline]
    pub fn retire_record(&mut self, record: Retired) {
        // the head bag is guaranteed to never be full
        unsafe { self.head.retired.push_unchecked(record) };
        if self.head.retired.is_full() {
            self.swap_head();
        }
    }

    /// Retires the [`ThreadState`] of an exiting thread as the final retire
    /// operation of that thread.
    ///
    /// # Safety
    ///
    /// After calling this method, no further calls to [`retire_record`] must be
    /// made.
    #[inline]
    pub unsafe fn retire_thread_state(&mut self, state: NonNull<Node<ThreadState>>) {
        self.head.retired.push_unchecked(Retired::new_unchecked(state));
    }

    /// # Safety
    ///
    /// It must be ensured that the contents of the queue are at least two
    /// epochs old.
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

/// A type-erased fat pointer to a retired record.
pub(crate) struct Retired(NonNull<dyn Any + 'static>);

impl Retired {
    /// # Safety
    ///
    /// ...
    #[inline]
    pub unsafe fn new_unchecked<'a, T: 'a>(record: NonNull<T>) -> Self {
        let any: NonNull<dyn Any + 'a> = record;
        let any: NonNull<dyn Any + 'static> = mem::transmute(any);

        Self(any)
    }

    /// Returns the memory address of the retired record.
    #[inline]
    pub fn address(&self) -> usize {
        self.0.as_ptr() as *const _ as *const () as usize
    }

    /// Reclaims the retired record.
    #[inline]
    unsafe fn reclaim(&mut self) {
        mem::drop(Box::from_raw(self.0.as_ptr()));
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
