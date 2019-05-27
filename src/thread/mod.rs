//! Global and local thread state
//!
//! Each thread has:
//!   - limbo bag (streaming) iterator: 3 bags, 1 always current
//!   - thread state iterator
//!   - operations counter
//!
//! On creation:
//!   - allocate global thread-state
//!   - insert into global set (based on heap address)
//!
//! On destruction:
//!   - mark current global epoch
//!   - remove own entry from global set
//!   - retire in current epoch's limbo bag
//!   - seal all limbo bags with current epoch + 2
//!   - push sealed bags on global stack
use core::mem::ManuallyDrop;

use typenum::U1;
use reclaim::align::CacheAligned;

use crate::retired::BagQueue;

mod set;

use self::set::{ThreadState, ThreadNode};

struct Local {
    state: ThreadState,
    inner: LocalInner,
}

struct LocalInner {
    ops_count: u32,
    bags: EpochBags,
    // thread_iter: ThreadStateIter,
}

struct EpochBags(ManuallyDrop<[BagQueue; 3]>);

