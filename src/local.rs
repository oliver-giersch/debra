//! Thread local state

use core::cell::{Cell, UnsafeCell};
use core::mem::ManuallyDrop;
use core::sync::atomic::Ordering;

use crate::epoch::{Epoch, State, ThreadState};
use crate::global;
use crate::retired::{Retired, SealedQueue};

pub(crate) use self::bag::SealedEpochBags;

use self::bag::EpochBags;

type ThreadEntry = crate::list::ListEntry<ThreadState>;
type ThreadStateIter = crate::list::Iter<'static, ThreadState>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Local
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Thread local state required for distributed epoch-based reclamation.
#[derive(Debug)]
pub struct Local {
    state: ManuallyDrop<ThreadEntry>,
    guard_count: Cell<usize>,
    inner: UnsafeCell<LocalInner>,
}

impl Local {
    /// Creates and globally registers a new [`Local`].
    #[inline]
    pub fn new() -> Self {
        let global_epoch = global::EPOCH.load(Ordering::SeqCst);
        let thread_epoch = ThreadState::new(global_epoch);
        let state = global::THREADS.insert(thread_epoch);

        Self {
            state: ManuallyDrop::new(state),
            guard_count: Cell::default(),
            inner: UnsafeCell::new(LocalInner::new(global_epoch)),
        }
    }

    /// Marks the associated thread as active.
    #[inline]
    pub(crate) fn set_active(&self) {
        let count = self.guard_count.get();
        if count == 0 {
            let inner = unsafe { &mut *self.inner.get() };
            inner.set_active(&**self.state);
        }

        self.guard_count.set(count + 1);
    }

    /// Marks the associated thread as inactive.
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
        inner.bags.retire_record(record);
    }
}

impl Drop for Local {
    #[inline]
    fn drop(&mut self) {
        let state = unsafe { ManuallyDrop::take(&mut self.state) };
        let entry = global::THREADS.remove(state);

        unsafe {
            let inner = &mut *self.inner.get();
            inner.bags.retire_thread_state(entry);
        }
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

            self.adopt_and_reclaim();
            unsafe { self.bags.rotate_and_reclaim() };
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

    #[inline]
    fn adopt_and_reclaim(&mut self) {
        for queues in global::ABANDONED.pop_all() {
            for sealed in queues {
                let x: SealedQueue = sealed;

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// EpochBags
////////////////////////////////////////////////////////////////////////////////////////////////////

mod bag {
    use core::ptr::NonNull;

    use arrayvec::ArrayVec;

    use crate::epoch::{Epoch, ThreadState};
    use crate::list::Node;
    use crate::retired::{BagQueue, Retired, SealedQueue};

    const BAG_COUNT: usize = 3;

    /// An array with **up to** three [`SealedQueue`]s.
    pub(crate) type SealedEpochBags = ArrayVec<[SealedQueue; BAG_COUNT]>;

    #[derive(Debug)]
    pub(super) struct EpochBags {
        curr_idx: usize,
        queues: [BagQueue; BAG_COUNT],
    }

    impl EpochBags {
        /// Creates a new empty set of [`EpochBags`].
        #[inline]
        pub fn new() -> Self {
            Self { curr_idx: 0, queues: [BagQueue::new(), BagQueue::new(), BagQueue::new()] }
        }

        /// Converts the three bag queues into **up to** three non-empty
        /// [`SealedQueues`].
        #[inline]
        pub fn into_sealed(self, current_epoch: Epoch) -> ArrayVec<[SealedQueue; BAG_COUNT]> {
            self.into_sorted()
                .into_iter()
                .enumerate()
                .filter_map(|(idx, queue)| queue.non_empty().map(|queue| (idx, queue)))
                .map(|(idx, queue)| queue.seal(current_epoch - idx))
                .collect()
        }

        /// Retires the given `record` in the current [`BagQueue`].
        #[inline]
        pub fn retire_record(&mut self, record: Retired) {
            let curr = &mut self.queues[self.curr_idx];
            curr.retire_record(record);
        }

        #[inline]
        pub unsafe fn retire_thread_state(&mut self, state: NonNull<Node<ThreadState>>) {
            let curr = &mut self.queues[self.curr_idx];
            curr.retire_thread_state(state);
        }

        #[inline]
        pub unsafe fn rotate_and_reclaim(&mut self) {
            self.curr_idx = (self.curr_idx + 1) % BAG_COUNT;
            self.queues[self.curr_idx].reclaim_full_bags();
        }

        #[inline]
        fn into_sorted(self) -> ArrayVec<[BagQueue; BAG_COUNT]> {
            let [a, b, c] = self.queues;
            match self.curr_idx {
                0 => ArrayVec::from([a, c, b]),
                1 => ArrayVec::from([b, a, c]),
                2 => ArrayVec::from([c, b, a]),
                _ => unreachable!(),
            }
        }
    }
}
