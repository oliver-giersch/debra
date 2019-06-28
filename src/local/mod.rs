//! Thread local state

mod inner;

use core::cell::{Cell, UnsafeCell};
use core::mem::ManuallyDrop;
use core::ptr;
use core::sync::atomic::Ordering;

use debra_common::thread::ThreadState;
use debra_common::LocalAccess;

use crate::global;
use crate::{Debra, Retired};

use self::inner::LocalInner;

type ThreadEntry = crate::list::ListEntry<'static, ThreadState>;

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
}

impl<'a> LocalAccess for &'a Local {
    type Reclaimer = Debra;

    #[inline]
    fn set_active(self) {
        let count = self.guard_count.get();
        if count == 0 {
            let inner = unsafe { &mut *self.inner.get() };
            inner.set_active(&**self.state);
        }

        self.guard_count.set(count + 1);
    }

    #[inline]
    fn set_inactive(self) {
        let count = self.guard_count.get();
        if count == 1 {
            let inner = unsafe { &*self.inner.get() };
            inner.set_inactive(&**self.state);
        }

        self.guard_count.set(count - 1);
    }

    #[inline]
    fn retire_record(self, record: Retired) {
        let inner = unsafe { &mut *self.inner.get() };
        inner.retire_record(record);
    }
}

impl Default for Local {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Local {
    #[inline]
    fn drop(&mut self) {
        let state = unsafe { take_manually_drop(&mut self.state) };
        let entry = global::THREADS.remove(state);

        unsafe {
            let retired = Retired::new_unchecked(entry);
            let inner = &mut *self.inner.get();
            inner.retire_final_record(retired);
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// helper functions
////////////////////////////////////////////////////////////////////////////////////////////////////

// this helper function can be removed when `ManuallyDrop::take` becomes stable
#[inline]
unsafe fn take_manually_drop<T>(slot: &mut ManuallyDrop<T>) -> T {
    ManuallyDrop::into_inner(ptr::read(slot))
}
