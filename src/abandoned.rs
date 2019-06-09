//! Simple concurrent & lock-free multiple-producer/single-consumer queue for
//! storing bag queues of exited threads.

use core::ptr::{self, NonNull};
use core::sync::atomic::{
    AtomicPtr,
    Ordering::{Acquire, Relaxed, Release},
};

use crate::bag::SealedSubList;

////////////////////////////////////////////////////////////////////////////////////////////////////
// AbandonedQueue
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub(crate) struct AbandonedQueue {
    head: AtomicPtr<Node>,
}

impl AbandonedQueue {
    /// Creates a new empty [`AbandonedQueue`].
    #[inline]
    pub const fn new() -> Self {
        Self { head: AtomicPtr::new(ptr::null_mut()) }
    }

    /// Push a new [`SealedEpochBags`] to the front of the queue.
    #[inline]
    pub fn push(&self, sealed: SealedEpochBags) {
        let node = Box::leak(Box::new(Node::new(sealed)));

        loop {
            let head = self.head.load(Relaxed);
            node.next = NonNull::new(head);

            if self.head.compare_exchange_weak(head, node, Release, Relaxed).is_ok() {
                return;
            }
        }
    }

    /// Pops the entire queue, returning an iterator over the popped elements.
    #[inline]
    pub fn pop_all(&self) -> SealedIter {
        let queue = NonNull::new(self.head.swap(ptr::null_mut(), Acquire));
        SealedIter { curr: queue }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// SealedIter
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub(crate) struct SealedIter {
    curr: Option<NonNull<Node>>,
}

impl Iterator for SealedIter {
    type Item = SealedEpochBags;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            self.curr.map(|ptr| {
                let curr = Box::from_raw(ptr.as_ptr());
                self.curr = curr.next;

                curr.sealed
            })
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
struct Node {
    sealed: SealedEpochBags,
    next: Option<NonNull<Node>>,
}

impl Node {
    fn new(sealed: SealedEpochBags) -> Self {
        Self { sealed, next: Option::default() }
    }
}
