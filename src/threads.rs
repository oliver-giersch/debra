//! A concurrent lock-free intrusive set for storing the (global) state of all
//! participating threads.

use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::Ordering::{self, Acquire, Relaxed};

use typenum::U1;
use reclaim::align::CacheAligned;
use reclaim::{Marked, MarkedPtr, MarkedPointer};

use crate::epoch::ThreadEpoch;
use crate::{Shared, Unprotected};
use std::path::Iter;

type Atomic<T> = crate::Atomic<T, U1>;
type Owned<T> = crate::Owned<T, U1>;
type Unlinked<T> = crate::Unlinked<T, U1>;

////////////////////////////////////////////////////////////////////////////////
// OrderedThreadSet
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub(crate) struct ThreadSet {
    head: Atomic<ThreadNode>,
}

impl ThreadSet {
    #[inline]
    pub fn insert(&self, entry: Owned<ThreadNode>) -> ThreadEntry {
        unimplemented!()
    }

    #[inline]
    pub fn remove(&self, entry: ThreadEntry) -> Unlinked<ThreadNode> {
        unimplemented!()
    }
}

////////////////////////////////////////////////////////////////////////////////
// ThreadIter
////////////////////////////////////////////////////////////////////////////////

pub(crate) struct ThreadIter<'a>(ThreadIterInner<'a>);

impl ThreadIter<'_> {
    #[inline]
    pub fn load_current(&self, order: Ordering) -> Option<Shared<ThreadNode>> {
        unsafe {
            let prev = &*self.0.prev.as_ptr();
            prev.load_unprotected(order).map(|curr| Unprotected::into_shared(curr))
        }
    }

    #[inline]
    pub fn advance(&mut self) {
        let _ = self.0.next();
    }
}

// TODO: How to iterate safely?
struct ThreadIterInner<'a> {
    head: &'a Atomic<ThreadNode>,
    prev: NonNull<Atomic<ThreadNode>>
}

impl ThreadIterInner {
    #[inline]
    fn next(&mut self) -> Option<IterPositon> {
        let prev = unsafe { &*self.prev.as_ptr() };
        while let Some(curr) = prev.load_unprotected(Acquire) {
            let (curr, curr_tag) = unsafe { Shared::decompose(Unprotected::into_shared(curr)) };
            if curr_tag == 0b1 {
                continue;
            }

            let next = curr.next.load_marked_unprotected(Acquire);

            if prev.load_raw(Relaxed) == MarkedPtr::from(curr) {
                continue;
            }

            self.prev = NonNull::from(&curr.next);
            return Some(IterPositon {
                prev,
                curr,
                next: next.map(|unprotected| unsafe { Unprotected::into_shared(unprotected) })
            });
        }

        None
    }
}

struct IterPositon<'iter> {
    prev: &'iter Atomic<ThreadNode>,
    curr: Shared<'iter, ThreadNode>,
    next: Marked<Shared<'iter, ThreadNode>>,
}

////////////////////////////////////////////////////////////////////////////////
// ThreadNode
////////////////////////////////////////////////////////////////////////////////

pub(crate) struct ThreadNode {
    epoch: CacheAligned<ThreadEpoch>,
    next: CacheAligned<Atomic<ThreadNode>>,
}

impl ThreadNode {
    #[inline]
    pub fn new(epoch: ThreadEpoch) -> Self {
        Self {
            epoch: CacheAligned(epoch),
            next: CacheAligned(Atomic::null()),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// ThreadEntry
////////////////////////////////////////////////////////////////////////////////

/// A token representing responsibility for an entry in the thread state set.
pub(crate) struct ThreadEntry(*const ThreadNode);

impl Deref for ThreadEntry {
    type Target = ThreadNode;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unimplemented!()
    }
}