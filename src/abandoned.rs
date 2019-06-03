//! Simple concurrent & lock-free multiple-producer/single-consumer queue for
//! storing bag queues of exited threads.

use crate::epoch::Epoch;

pub(crate) struct AbandonedQueue;
