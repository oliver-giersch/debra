//! Simple concurrent & lock-free multiple-producer/single-consumer queue for
//! storing bag queues of exited threads.

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use core::ptr::{self, NonNull};
use core::sync::atomic::{
    AtomicPtr,
    Ordering::{Acquire, Relaxed, Release},
};

use crate::sealed::{Sealed, SealedList};

////////////////////////////////////////////////////////////////////////////////////////////////////
// AbandonedQueue
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A LIFO-queue (stack) that allows concurrent push and pop operations, which
/// take the entire content of the stack at once.
#[derive(Debug)]
pub(crate) struct AbandonedQueue {
    head: AtomicPtr<Sealed>,
}

/***** impl inherent ******************************************************************************/

impl AbandonedQueue {
    /// Creates a new empty [`AbandonedQueue`].
    #[inline]
    pub const fn new() -> Self {
        Self { head: AtomicPtr::new(ptr::null_mut()) }
    }

    /// Push a new [`SealedEpochBags`] to the front of the queue.
    #[inline]
    pub fn push(&self, sealed: SealedList) {
        let (head, mut tail) = sealed.into_inner();

        loop {
            let curr_head = self.head.load(Relaxed);
            unsafe { tail.as_mut().next = NonNull::new(curr_head) };

            // (ABA:1) this `Release` CAS synchronizes-with the `Acquire` swap (ABA:2)
            if self.head.compare_exchange_weak(curr_head, head.as_ptr(), Release, Relaxed).is_ok() {
                return;
            }
        }
    }

    /// Pops the entire queue, returning an [`Iter`] over the popped elements.
    #[inline]
    pub fn take_all(&self) -> Iter {
        // (ABA:2) this `Acquire` swap synchronizes-with the `Release` CAS (ABA:1)
        let head = self.head.swap(ptr::null_mut(), Acquire);
        Iter { curr: NonNull::new(head) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Iter
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An iterator over the results of a [`take_all`][AbandonedQueue::take_all]
/// call.
#[derive(Debug)]
pub(crate) struct Iter {
    curr: Option<NonNull<Sealed>>,
}

/***** impl Iterator ******************************************************************************/

impl Iterator for Iter {
    type Item = Box<Sealed>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            self.curr.map(|ptr| {
                let curr = Box::from_raw(ptr.as_ptr());
                self.curr = curr.next;

                curr
            })
        }
    }
}
