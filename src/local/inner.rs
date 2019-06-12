use core::mem::ManuallyDrop;
use core::sync::atomic::Ordering::{Acquire, Release, SeqCst};

use crate::bag::{BagPool, EpochBagQueues};
use crate::epoch::{Epoch, State, ThreadState};
use crate::global;
use crate::retired::Retired;

type ThreadStateIter = crate::list::Iter<'static, ThreadState>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// LocalInner
////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
pub(super) struct LocalInner {
    bags: ManuallyDrop<EpochBagQueues>,
    bag_pool: BagPool,
    cached_local_epoch: Epoch,
    can_advance: bool,
    check_count: u32,
    ops_count: u32,
    thread_iter: ThreadStateIter,
}

impl LocalInner {
    const CHECK_THRESHOLD: u32 = 100;
    const ADVANCE_THRESHOLD: u32 = 100;

    #[inline]
    pub fn new(global_epoch: Epoch) -> Self {
        Self {
            bags: ManuallyDrop::new(EpochBagQueues::new()),
            bag_pool: BagPool::new(),
            cached_local_epoch: global_epoch,
            can_advance: false,
            check_count: 0,
            ops_count: 0,
            thread_iter: global::THREADS.iter(),
        }
    }

    #[inline]
    pub fn set_active(&mut self, thread_state: &ThreadState) {
        // (INN:1) this `Acquire` load synchronizes-with the `Release` CAS (INN:6)
        let global_epoch = global::EPOCH.load(Acquire);

        // the global epoch has been advanced, restart all incremental checks
        if global_epoch != self.cached_local_epoch {
            self.cached_local_epoch = global_epoch;
            self.can_advance = false;
            self.ops_count = 0;
            self.check_count = 0;
            self.thread_iter = global::THREADS.iter();

            unsafe { self.bags.rotate_and_reclaim(&mut self.bag_pool) };
            self.adopt_and_reclaim();
        }

        self.ops_count += 1;
        self.try_advance(thread_state, global_epoch);

        // (INN:2) this `SeqCst` store synchronizes-with the `SeqCst` load (INN:7), establishing a
        // total order of all operations on `ThreadState` values.
        thread_state.store(global_epoch, State::Active, SeqCst);
    }

    #[inline]
    pub fn set_inactive(&self, thread_state: &ThreadState) {
        // (INN:3) this `SeqCst` store synchronizes-with the `SeqCst` load (INN:7), establishing a
        // total order of all operations on `ThreadState` values.
        thread_state.store(self.cached_local_epoch, State::Quiescent, SeqCst);
    }

    #[inline]
    pub fn retire_record(&mut self, record: Retired) {
        self.bags.retire_record(record, &mut self.bag_pool);
    }

    #[inline]
    pub unsafe fn retire_final_record(&mut self, record: Retired) {
        self.bags.retire_final_record(record);
    }

    #[inline]
    fn try_advance(&mut self, thread_state: &ThreadState, global_epoch: Epoch) {
        if self.ops_count >= Self::CHECK_THRESHOLD {
            self.ops_count = 0;

            // (INN:4) this `Acquire` load the `Release` CAS (LIS:1) and (LIS:3)
            if let Ok(curr) = self.thread_iter.load_current(Acquire) {
                let other = curr.unwrap_or_else(|| {
                    self.can_advance = true;
                    self.thread_iter = global::THREADS.iter();
                    // (INN:5) this `Acquire` load synchronizes-with the the `Release` CAS (LIS:1) and (LIS:3)
                    self.thread_iter.load_head(Acquire).unwrap_or_else(|| unreachable!())
                });

                if thread_state.is_same(other) || can_advance(global_epoch, other) {
                    self.check_count += 1;
                    let _ = self.thread_iter.next();

                    // (INN:6) this `Release` CAS synchronizes-with the `Acquire` load (INN:1)
                    if self.can_advance && self.check_count >= Self::ADVANCE_THRESHOLD {
                        global::EPOCH.compare_and_swap(global_epoch, global_epoch + 1, Release);
                    }
                }
            }
        }
    }

    #[inline]
    fn adopt_and_reclaim(&mut self) {
        for sealed in global::ABANDONED.take_all() {
            self.bags.retire_sealed(self.cached_local_epoch, sealed, &mut self.bag_pool);
        }
    }
}

impl Drop for LocalInner {
    #[inline]
    fn drop(&mut self) {
        let bags = unsafe { ManuallyDrop::take(&mut self.bags) };
        if let Some(sealed) = bags.into_sealed(self.cached_local_epoch) {
            global::ABANDONED.push(sealed);
        }
    }
}

#[inline(always)]
fn can_advance(global_epoch: Epoch, other: &ThreadState) -> bool {
    // (INN:7) this `SeqCst` load synchronizes-with the `SeqCst` stores (INN:2) and (INN:3),
    // establishing
    let (epoch, is_active) = other.load_decompose(SeqCst);
    epoch == global_epoch || !is_active
}
