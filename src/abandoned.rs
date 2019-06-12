//! Simple concurrent & lock-free multiple-producer/single-consumer queue for
//! storing bag queues of exited threads.

use core::ptr::{self, NonNull};
use core::sync::atomic::{
    AtomicPtr,
    Ordering::{Acquire, Relaxed, Release},
};

use crate::bag::{SealedList, SealedQueue};

////////////////////////////////////////////////////////////////////////////////////////////////////
// AbandonedQueue
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A LIFO-queue (stack) that allows concurrent push and pop operations, which
/// take the entire content of the stack at once.
#[derive(Debug)]
pub(crate) struct AbandonedQueue {
    head: AtomicPtr<SealedQueue>,
}

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

    /// Pops the entire queue, returning an iterator over the popped elements.
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

#[derive(Debug)]
pub(crate) struct Iter {
    curr: Option<NonNull<SealedQueue>>,
}

impl Iterator for Iter {
    type Item = Box<SealedQueue>;

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
