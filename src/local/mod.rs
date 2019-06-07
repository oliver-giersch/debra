//! Thread local state

use core::cell::{Cell, UnsafeCell};
use core::mem::ManuallyDrop;
use core::sync::atomic::Ordering;

use crate::epoch::{Epoch, State, ThreadState};
use crate::global;
use crate::retired::{Retired, SealedQueue};

pub(crate) use self::bag::SealedEpochBags;

mod bag;
mod inner;

use self::inner::LocalInner;

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
