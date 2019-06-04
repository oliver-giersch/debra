//! The global static epoch counter

use crate::epoch::{AtomicEpoch, ThreadState};
use crate::list::List;

pub(crate) static EPOCH: AtomicEpoch = AtomicEpoch::new();
pub(crate) static THREADS: List<ThreadState> = List::new();
