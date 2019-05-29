//! Global concurrent intrusive linked-list based set for thread states with
//! dynamic memory reclamation.
//!
//! **MAYBE UNSOUND**
//!
//! - not sure if reclaiming with current bag of exiting thread ensures no other
//!   threads have e.g. an old iterator value
//! - maybe switch to free-list
use core::ops::Deref;

use typenum::U1;
use reclaim::align::CacheAligned;

use crate::epoch::ThreadEpoch;
use crate::{Atomic, Owned, Shared, Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////
// ThreadSet
////////////////////////////////////////////////////////////////////////////////

pub(crate) struct ThreadSet;

impl ThreadSet {
    #[inline]
    pub fn insert(&self, entry: Owned<ThreadNode, U1>) -> ThreadState {
        unimplemented!()
    }

    #[inline]
    pub fn remove(&self, entry: ThreadState) -> Unlinked<ThreadNode, U1> {
        unimplemented!()
    }
}

////////////////////////////////////////////////////////////////////////////////
// ThreadIter
////////////////////////////////////////////////////////////////////////////////

// TODO: How to iterate safely?
pub(crate) struct ThreadIter(Option<Unprotected<ThreadNode, U1>>);

impl ThreadIter {
    #[inline]
    pub fn get(&self) -> Option<&ThreadNode> {
        unimplemented!()
    }

    #[inline]
    pub fn next(&mut self) {
        unimplemented!()
    }
}

////////////////////////////////////////////////////////////////////////////////
// ThreadStateNode
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
// ThreadState
////////////////////////////////////////////////////////////////////////////////

/// A token representing responsibility for an entry in the thread state set.
pub(crate) struct ThreadState(Shared<'static, ThreadNode, U1>); // TODO: Make &'static ThreadNode?

impl Deref for ThreadState {
    type Target = ThreadNode;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}