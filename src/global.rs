//! The global static epoch counter

use crate::epoch::AtomicEpoch;

pub(crate) static EPOCH: AtomicEpoch = AtomicEpoch::new();
// pub(crate) static THREADS: ThreadSet = ThreadSet::new();