//! The global static epoch counter

use crate::abandoned::AbandonedQueue;
use crate::epoch::{AtomicEpoch, ThreadState};
use crate::list::List;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Global variables & data structures
////////////////////////////////////////////////////////////////////////////////////////////////////

pub(crate) static ABANDONED: AbandonedQueue = AbandonedQueue::new();
pub(crate) static EPOCH: AtomicEpoch = AtomicEpoch::new();
pub(crate) static THREADS: List<ThreadState> = List::new();
