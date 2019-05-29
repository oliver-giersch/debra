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
//!
//! To mark active:
//!
//!     ```ignore
//!     let global_epoch = GLOBAL_EPOCH.load();
//!     let local_epoch = state.cached_local_epoch(); // access TLS for cached value
//!
//!     if global_epoch != local_epoch {
//!         state.can_advance = false;
//!         state.ops_count = 0;
//!         state.check_count = 0;
//!         state.thread_iter = THREADS.iter();
//!         state.bags.rotate(); // change current, empty "new" current bag queue
//!     }
//!
//!     state.ops_count += 1;
//!     if state.ops_count >= THRESHOLD {
//!         state.ops_count = 0;
//!         let other = state.thread_iter.prev.load().unwrap_or_else(|| {
//!             let head = THREADS.iter();
//!             state.can_advance = true;
//!             state.thread_iter.prev = head;
//!             head.load().unwrap_or_else(|| unreachable!())
//!         });
//!
//!         let (other, tag) = Shared::decompose_ref(other);
//!         if tag != REMOVE_TAG {
//!             if check_try_advance_conditions(global_epoch, thread_state, other) {
//!                 state.thread_iter.prev = &*other.next;
//!                 state.check_count += 1;
//!                 if state.can_advance && state.check_count >= CHECK_THRESHOLD {
//!                     let _ = GLOBAL_EPOCH.compare_exchange(global_epoch, global_epoch.increment());
//!                 }
//!             }
//!         }
//!
//!         thread_state.epoch.store(global_epoch, false);
//!     ```
//!
//!     ```ignore
//!     fn check_advance_conditions(global_epoch: Epoch, thread_state: &ThreadState, other: &ThreadState) -> bool {
//!         if thread_state as *const _ == other as *const _ {
//!             return true;
//!         }
//!
//!         let (epoch, is_active) = other.epoch.decompose_load();
//!         if epoch == global_epoch || !is_active {
//!             return true;
//!         }
//!
//!         return false;
//!     }
//!     ```
use core::cell::{Cell, UnsafeCell};
use core::mem::ManuallyDrop;
use core::sync::atomic::Ordering;

use typenum::U1;
use reclaim::align::CacheAligned;

use crate::epoch::Epoch;
use crate::global;
use crate::retired::{BagQueue, Retired};

mod set;

use self::set::{ThreadState, ThreadNode};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Local
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Local {
    state: ThreadState,
    guard_count: Cell<usize>,
    inner: UnsafeCell<LocalInner>,
}

impl Local {
    #[inline]
    pub(crate) fn set_active(&self) {
        let count = self.guard_count.get();
        if count == 0 {
            let inner = unsafe { &mut *self.inner.get() };
            inner.set_active(&self.state);
        }

        self.guard_count.set(count + 1);
    }

    /// Retires an unlinked record in the current epoch's bag queue.
    #[inline]
    pub(crate) fn retire_record(&self, record: Retired) {
        unimplemented!()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// LocalInner
////////////////////////////////////////////////////////////////////////////////////////////////////

struct LocalInner {
    can_advance: bool,
    ops_count: u32,
    check_count: u32,
    cached_local_epoch: Epoch,
    bags: EpochBags,
    // thread_iter: ThreadStateIter,
}

impl LocalInner {
    const CHECK_THRESHOLD: u32 = 100;
    const ADVANCE_THRESHOLD: u32 = 100;

    #[inline]
    fn set_active(&mut self, thread_state: &ThreadState) {
        let global_epoch = global::EPOCH.load(Ordering::SeqCst);

        // the global epoch has been advanced, all incremental checks restart
        if global_epoch != self.cached_local_epoch {
            self.can_advance = false;
            self.ops_count = 0;
            self.check_count = 0;
            // self.thread_iter = THREADS.iter();
            // self.bags.rotate_and_reclaim(); // change current, empty "new" current bag queue
        }

        self.ops_count += 1;
        if self.ops_count >= Self::CHECK_THRESHOLD {
            self.ops_count = 0;

            // need self.thread_iter here ...
            // let other: Shared<'iter, ThreadNode> = self
            //     .thread_iter
            //     .load_current(SeqCst)
            //     .unwrap_or_else(|| {
            //         self.can_advance = true;
            //         let iter = global::THREADS.iter();
            //         iter.load_current(SeqCst).unwrap_or_else(|| unreachable!())
            //     });
            //
            // let (other, tag) = Shared::decompose_ref(other);
            // if tag != DELETE_TAG {
            //     if is_same(other, thread_state) || thread_state.epoch.load_decompose(SeqCst).can_advance(global_epoch) {
            //         self.check_count += 1;
            //         // curr may have been removed already, so we load again
            //         self.thread_iter.advance();
            //
            //         if self.check_count >= Self::ADVANCE_THRESHOLD && self.can_advance {
            //             let _ = global::EPOCH.compare_and_swap(global_epoch, global_epoch.increment(), SeqCst);
            //         }
            //     }
            // }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// EpochBags
////////////////////////////////////////////////////////////////////////////////////////////////////

struct EpochBags {
    queues: ManuallyDrop<[BagQueue; 3]>,
    current_idx: usize,
}

impl EpochBags {
    #[inline]
    fn retire_record(&mut self, record: Retired) {
        let queue = &mut *self.queues[self.current_idx];
        unimplemented!()
    }
}

