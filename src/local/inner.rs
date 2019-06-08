use core::mem::ManuallyDrop;
use core::sync::atomic::Ordering;

use crate::bag::{BagPool, EpochBagQueues};
use crate::epoch::{Epoch, State, ThreadState};
use crate::global;
use crate::retired::Retired;

type ThreadEntry = crate::list::ListEntry<ThreadState>;
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
        let global_epoch = global::EPOCH.load(Ordering::SeqCst);

        // the global epoch has been advanced, restart all incremental checks
        if global_epoch != self.cached_local_epoch {
            self.can_advance = false;
            self.ops_count = 0;
            self.check_count = 0;
            self.thread_iter = global::THREADS.iter();

            self.adopt_and_reclaim();
            unsafe { self.bags.rotate_and_reclaim(&mut self.bag_pool) };
        }

        self.ops_count += 1;
        self.try_advance(thread_state, global_epoch);

        thread_state.store(global_epoch, State::Active, Ordering::SeqCst);
        self.cached_local_epoch = global_epoch;
    }

    #[inline]
    pub fn set_inactive(&self, thread_state: &ThreadState) {
        thread_state.store(self.cached_local_epoch, State::Quiescent, Ordering::SeqCst);
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
                        global::EPOCH.compare_and_swap(
                            global_epoch,
                            global_epoch.increment(),
                            Ordering::SeqCst,
                        );
                    }
                }
            }
        }
    }

    #[inline]
    fn adopt_and_reclaim(&mut self) {
        for queues in global::ABANDONED.pop_all() {
            for sealed in queues {
                // let x: SealedQueue = sealed;
                // if epoch - 2 > sealed.epoch -> reclaim right away
                // else retire in appropriate bag FIXME: can not do...
            }
        }

        let mut abandonned = global::ABANDONED.pop_all();
    }
}

impl Drop for LocalInner {
    #[inline]
    fn drop(&mut self) {
        let bags = unsafe { ManuallyDrop::take(&mut self.bags) };
        global::ABANDONED.push(bags.into_sealed(self.cached_local_epoch));
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
