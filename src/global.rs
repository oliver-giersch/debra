//! The global static epoch counter

use crate::epoch::AtomicEpoch;

static GLOBAL_EPOCH: AtomicEpoch = AtomicEpoch::new();