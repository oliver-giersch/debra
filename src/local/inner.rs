#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use core::mem::ManuallyDrop;
use core::ptr::{self, NonNull};
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst};

use debra_common::epoch::Epoch;
use debra_common::thread::{
    State::{Active, Inactive},
    ThreadState,
};

use crate::config::Config;
use crate::global::{ABANDONED, CONFIG, EPOCH, THREADS};
use crate::sealed::SealedList;
use crate::Retired;

type BagPool = debra_common::bag::BagPool<crate::Debra>;
type EpochBagQueues = debra_common::bag::EpochBagQueues<crate::Debra>;
type ThreadStateIter = crate::list::Iter<'static, ThreadState>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// LocalInner
////////////////////////////////////////////////////////////////////////////////////////////////////

/// The internal mutable thread-local state.
#[derive(Debug)]
pub(super) struct LocalInner {
    bags: ManuallyDrop<EpochBagQueues>,
    bag_pool: BagPool,
    cached_local_epoch: Epoch,
    can_advance: bool,
    check_count: u32,
    config: Config,
    ops_count: u32,
    thread_iter: ThreadStateIter,
}

/***** impl inherent ******************************************************************************/

impl LocalInner {
    /// Creates a new [`LocalInner`].
    #[inline]
    pub fn new(global_epoch: Epoch) -> Self {
        Self {
            bags: ManuallyDrop::new(EpochBagQueues::new()),
            bag_pool: BagPool::new(),
            cached_local_epoch: global_epoch,
            can_advance: false,
            check_count: 0,
            config: CONFIG.read_config_or_default(),
            ops_count: 0,
            thread_iter: THREADS.iter(),
        }
    }

    /// Attempts to reclaim the retired records in the oldest epoch bag queue.
    #[inline]
    pub fn try_flush(&mut self, thread_state: &ThreadState) {
        let global_epoch = self.acquire_and_assess_global_epoch();

        if self.cached_local_epoch != global_epoch {
            // irrelevant for other threads since the thread remains inactive
            thread_state.store(global_epoch, Inactive, Relaxed);
        }
    }

    /// Marks the associated thread as active.
    #[inline]
    pub fn set_active(&mut self, thread_state: &ThreadState) {
        let global_epoch = self.acquire_and_assess_global_epoch();

        //self.ops_count += 1;
        self.ops_count = self.ops_count.wrapping_add(1);
        if self.ops_count % 128 == 0 {
            //self.config.check_threshold() {
            //self.ops_count = 0;
            self.try_advance(thread_state, global_epoch);
        }

        // (INN:1) this `SeqCst` store synchronizes-with the `SeqCst` load (INN:5), establishing a
        // total order of all operations on `ThreadState` values.
        // this operation announces the current global epoch and marks the thread as active to all
        // other threads, the cached epoch is only updated in the next call to set_active
        thread_state.store(global_epoch, Active, SeqCst);
    }

    /// Marks the associated thread as inactive.
    #[inline]
    pub fn set_inactive(&self, thread_state: &ThreadState) {
        // (INN:2) this `Release` store synchronizes-with the `SeqCst` load (INN:5) but without
        // partaking in the total order of operations on `ThreadState` values.
        thread_state.store(self.cached_local_epoch, Inactive, Release);
    }

    /// Retires the given `record` in the current epoch's bag queue.
    #[inline]
    pub fn retire_record(&mut self, record: Retired) {
        self.bags.retire_record(record, &mut self.bag_pool);
    }

    /// Retires the given `record` in the current epoch's bag queue as the final
    /// record of an exiting thread.
    ///
    /// # Safety
    ///
    /// After calling this method, no further calls to `retire_record` or
    /// `retire_final_record` must be made.
    #[inline]
    pub unsafe fn retire_final_record(&mut self, record: Retired) {
        self.bags.retire_final_record(record);
    }

    /// Loads ([`Acquire`]) the global epoch and compares it with the local one.
    ///
    /// If the local epoch is older than the global epoch, all incremental
    /// checks are restarted and all full bags in the oldest epoch bag queue
    /// are reclaimed.
    #[inline]
    fn acquire_and_assess_global_epoch(&mut self) -> Epoch {
        // (INN:3) this `Acquire` load synchronizes-with the `Release` CAS (INN:4)
        let global_epoch = EPOCH.load(Acquire);

        // the global epoch has been advanced since the last time this thread has called
        // `set_active`, restart all incremental checks
        if self.cached_local_epoch != global_epoch {
            self.cached_local_epoch = global_epoch;
            self.can_advance = false;
            self.ops_count = 0;
            self.check_count = 0;
            self.thread_iter = THREADS.iter();

            unsafe { self.rotate_and_reclaim() };
        }

        global_epoch
    }

    /// Attempts to advance the global epoch.
    ///
    /// The global epoch can only be advanced, if all currently active threads
    /// have been visited at least once.
    /// Each call of `try_advance` only visits exactly one thread.
    /// The iterator over all registered threads can only advance, if a visited
    /// thread is either currently not active or has itself previously announced
    /// the current global epoch.
    /// If the a thread visits its own entry, the entry is skipped and the
    /// iterator is likewise advanced.
    ///
    /// Only, once a thread has visited all threads at least once and has
    /// observed all threads in a valid state (i.e. either inactive or as having
    /// announced the global epoch), it can attempt to advance the global epoch.
    #[cold]
    fn try_advance(&mut self, thread_state: &ThreadState, global_epoch: Epoch) {
        if let Ok(curr) = self.thread_iter.load_current_acquire() {
            let other = curr.unwrap_or_else(|| {
                // we reached the end of the list and can restart, since this means we have
                // successfully advanced over all other threads in the list at least once;
                // if new threads have spawned (and been inserted at the front of the list), these
                // must have started in the global epoch, so we know it is safe to advance
                self.can_advance = true;
                self.thread_iter = THREADS.iter();
                // at least the current thread is still alive, so the thread list can not be empty
                self.thread_iter.load_head_acquire().unwrap_or_else(|| unreachable!())
            });

            // the iterator can only be advanced if the currently observed thread is either
            //   a) the same as the observing thread (us),
            //   b) has announced the global epoch or
            //   c) is currently inactive
            if thread_state.is_same(other) || can_advance(global_epoch, other) {
                self.check_count += 1;
                let _ = self.thread_iter.next();

                // we must have checked all other threads at least once, before we can attempt to
                // advance the global epoch
                if self.can_advance && self.check_count >= self.config.advance_threshold() {
                    // (INN:4) this `Release` CAS synchronizes-with the `Acquire` load (INN:3)
                    EPOCH.compare_and_swap(global_epoch, global_epoch + 1, Release);
                }
            }
        }
    }

    /// Checks if there are any abandoned records from threads that have quit.
    ///
    /// Any records that are older than two epochs are immediately reclaimed,
    /// all others are put in the local thread's appropriate epoch bags
    #[cold]
    unsafe fn rotate_and_reclaim(&mut self) {
        // reclaims the oldest retired records and rotates the queues so that further records are
        // retired into the flushed queue
        self.bags.rotate_and_reclaim(&mut self.bag_pool);

        // after rotating the epoch bags, we can potentially insert abandoned bags into their
        // appropriate queues (this must only be done AFTER the rotation!)
        for sealed in ABANDONED.take_all() {
            // sealed bags are retired according to the already adjusted epoch, otherwise they
            // are dropped and their contents reclaimed right away
            if let Ok(age) = sealed.seal.relative_age(self.cached_local_epoch) {
                let retired = Retired::new_unchecked(NonNull::from(Box::leak(sealed)));
                self.bags.retire_record_by_age(retired, age, &mut self.bag_pool);
            }
        }
    }
}

/***** impl Drop **********************************************************************************/

impl Drop for LocalInner {
    #[inline]
    // when a thread exits or panics, its yet un-reclaimed records can neither be leaked nor
    // instantly reclaimed; instead, all non-empty bag queues are pushed into a global queue, from
    // where other threads can adopt them and integrate them into their own appropriate epoch bags.
    fn drop(&mut self) {
        let bags = unsafe { ptr::read(&*self.bags) };
        if let Some(sealed) = SealedList::from_bags(bags, self.cached_local_epoch) {
            ABANDONED.push(sealed);
        }
    }
}

/***** helper functions ***************************************************************************/

/// A visiting thread can advance its local thread iterator if the visited
/// thread is either inactive or has itself announced the global epoch.
#[inline(always)]
fn can_advance(global_epoch: Epoch, other: &ThreadState) -> bool {
    // (INN:5) this `SeqCst` load synchronizes-with the `SeqCst` stores (INN:1) and (INN:2),
    // establishing a total order of all operations on `ThreadState` values.
    let (epoch, state) = other.load(SeqCst);
    epoch == global_epoch || state == Inactive
}
