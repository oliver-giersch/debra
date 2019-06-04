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

use core::cell::{Cell, UnsafeCell};
use core::mem::ManuallyDrop;
use core::ptr;
use core::sync::atomic::Ordering;

use crate::epoch::{Epoch, State, ThreadState};
use crate::global;
use crate::retired::{BagQueue, Retired};

type ThreadStateIter = crate::list::Iter<'static, ThreadState>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Local
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Local {
    state: ManuallyDrop<ListEntry<ThreadState>>,
    guard_count: Cell<usize>,
    inner: UnsafeCell<LocalInner>,
}

impl Local {
    /// TODO: Doc...
    #[inline]
    pub fn new() -> Self {
        let global_epoch = global::EPOCH.load(Ordering::SeqCst);
        let thread_epoch = ThreadState::new(global_epoch);
        let state = global::THREADS.insert(thread_epoch);

        Self {
            state,
            guard_count: Cell::default(),
            inner: UnsafeCell::new(LocalInner::new(global_epoch)),
        }
    }

    #[inline]
    pub(crate) fn set_active(&self) {
        let count = self.guard_count.get();
        if count == 0 {
            let inner = unsafe { &mut *self.inner.get() };
            inner.set_active(&**self.state);
        }

        self.guard_count.set(count + 1);
    }

    #[inline]
    pub(crate) fn set_inactive(&self) {
        let count = self.guard_count.get();
        if count == 1 {
            let inner = unsafe { &*self.inner.get() };
            inner.set_inactive(&**self.state);
        }

        self.guard_count.set(count - 1);
    }

    /// Retires an unlinked record in the current epoch's bag queue.
    #[inline]
    pub(crate) fn retire_record(&self, record: Retired) {
        let inner = unsafe { &mut *self.inner.get() };

        unimplemented!()
    }
}

impl Drop for Local {
    #[inline]
    fn drop(&mut self) {
        let state = unsafe { ManuallyDrop::take(&mut self.state) };
        let entry = global::THREADS.remove(state);
        let retired = unsafe { Retired::new_unchecked(entry) };
        // self.retire_record(retired); //TODO: retire_final_record ?
        unimplemented!()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// LocalInner
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
struct LocalInner {
    can_advance: bool,
    ops_count: u32,
    check_count: u32,
    cached_local_epoch: Epoch,
    bags: ManuallyDrop<EpochBags>,
    thread_iter: ThreadStateIter,
}

impl LocalInner {
    const CHECK_THRESHOLD: u32 = 100;
    const ADVANCE_THRESHOLD: u32 = 100;

    #[inline]
    fn new(global_epoch: Epoch) -> Self {
        Self {
            can_advance: false,
            ops_count: 0,
            check_count: 0,
            cached_local_epoch: global_epoch,
            bags: ManuallyDrop::new(EpochBags::new()),
            thread_iter: global::THREADS.iter(),
        }
    }

    #[inline]
    fn set_active(&mut self, thread_state: &ThreadState) {
        let global_epoch = global::EPOCH.load(Ordering::SeqCst);

        // the global epoch has been advanced, restart all incremental checks
        if global_epoch != self.cached_local_epoch {
            self.can_advance = false;
            self.ops_count = 0;
            self.check_count = 0;
            self.thread_iter = global::THREADS.iter();
            self.bags.rotate_and_reclaim(); // change current, empty "new" current bag queue
        }

        self.ops_count += 1;
        let epoch = self.try_advance(thread_state, global_epoch);

        thread_state.store(epoch, State::Active, Ordering::SeqCst);
        self.cached_local_epoch = epoch;
    }

    #[inline]
    fn set_inactive(&self, thread_state: &ThreadState) {
        thread_state.store(self.cached_local_epoch, State::Quiescent, Ordering::SeqCst);
    }

    #[inline]
    fn try_advance(&mut self, thread_state: &ThreadState, global_epoch: Epoch) -> Epoch {
        if self.ops_count >= Self::CHECK_THRESHOLD {
            self.ops_count = 0;

            if let Ok(curr) = self.thread_iter.load_current(Ordering::SeqCst) {
                let other = curr.unwrap_or_else(|| {
                    self.can_advance = true;
                    self.thread_iter = global::THREADS.iter();
                    self.thread_iter.load_head(Ordering::SeqCst).unwrap_or_else(|| unreachable!())
                });

                if thread_state.is_same(other) || can_advance(global_epoch, other) {
                    self.check_count += 1;
                    let _ = self.thread_iter.next();

                    if self.can_advance && self.check_count >= Self::ADVANCE_THRESHOLD {
                        return global::EPOCH.compare_and_swap(
                            global_epoch,
                            global_epoch.increment(),
                            Ordering::SeqCst,
                        );
                    }
                }
            }
        }

        global_epoch
    }
}

#[inline(always)]
fn can_advance(global_epoch: Epoch, other: &ThreadState) -> bool {
    let (epoch, is_active) = other.load_decompose(Ordering::SeqCst);
    if epoch == global_epoch || !is_active {
        true
    } else {
        false
    }
}

impl Drop for LocalInner {
    #[inline]
    fn drop(&mut self) {
        // global::ABANDONED.abandon_epoch_bags(self.epoch_bags);
        unimplemented!()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// EpochBags
////////////////////////////////////////////////////////////////////////////////////////////////////

const BAG_COUNT: usize = 3;

#[derive(Debug)]
struct EpochBags {
    current_idx: usize,
    queues: [BagQueue; BAG_COUNT],
}

impl EpochBags {
    #[inline]
    fn rotate_and_reclaim(&mut self) {
        self.current_idx = (self.current_idx + 1) % BAG_COUNT;
        unsafe { self.queues[self.current_idx].reclaim_full_bags() };
    }

    #[inline]
    fn retire_record(&mut self, record: Retired) {
        let curr = &mut self.queues[self.current_idx];
        curr.retire_record(record);
    }
}
