//! Global concurrent intrusive linked-list based set for thread states with
//! dynamic memory reclamation.
use core::ops::Deref;

use reclaim::align::CacheAligned;

use crate::epoch::ThreadEpoch;
use crate::{Atomic, Owned, Shared, Unlinked};

////////////////////////////////////////////////////////////////////////////////
// ThreadStateSet
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

// TODO: How to iterate safely?

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