//! Global (static) values and data structures.

use debra_common::epoch::AtomicEpoch;
use debra_common::thread::ThreadState;

use crate::abandoned::AbandonedQueue;
use crate::config::GlobalConfig;
use crate::list::List;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Global variables & data structures
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Global configuration for the reclamation scheme.
///
/// Can only be set once during the runtime of a program.
pub static CONFIG: GlobalConfig = GlobalConfig::new();

pub(crate) static ABANDONED: AbandonedQueue = AbandonedQueue::new();
pub(crate) static EPOCH: AtomicEpoch = AtomicEpoch::new();
pub(crate) static THREADS: List<ThreadState> = List::new();
