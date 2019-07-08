//! Global (static) values and data structures.

use debra_common::epoch::AtomicEpoch;
use debra_common::thread::ThreadState;

use crate::abandoned::AbandonedQueue;
use crate::list::List;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Global variables & data structures
////////////////////////////////////////////////////////////////////////////////////////////////////

pub(crate) static ABANDONED: AbandonedQueue = AbandonedQueue::new();
pub(crate) static EPOCH: AtomicEpoch = AtomicEpoch::new();
pub(crate) static THREADS: List<ThreadState> = List::new();
