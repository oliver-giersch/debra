//! Thread local state

mod inner;

use core::cell::{Cell, UnsafeCell};
use core::mem::ManuallyDrop;
use core::ptr;
use core::sync::atomic::Ordering;

use debra_common::thread::ThreadState;
use debra_common::LocalAccess;

use crate::global::{EPOCH, THREADS};
use crate::{Debra, Retired};

use self::inner::LocalInner;

type ThreadEntry = crate::list::ListEntry<'static, ThreadState>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Local
////////////////////////////////////////////////////////////////////////////////////////////////////

/// The thread-local state required for distributed epoch-based reclamation.
#[derive(Debug)]
pub struct Local {
    state: ManuallyDrop<ThreadEntry>,
    guard_count: Cell<usize>,
    inner: UnsafeCell<LocalInner>,
}

/***** impl inherent ******************************************************************************/

impl Local {
    /// Creates and globally registers a new [`Local`].
    pub fn new() -> Self {
        let global_epoch = EPOCH.load(Ordering::SeqCst);
        let thread_epoch = ThreadState::new(global_epoch);
        let state = THREADS.insert(thread_epoch);

        Self {
            state: ManuallyDrop::new(state),
            guard_count: Cell::default(),
            inner: UnsafeCell::new(LocalInner::new(global_epoch)),
        }
    }

    /// Attempts to reclaim the retired records in the oldest epoch bag queue.
    #[inline]
    pub fn try_flush(&self) {
        unsafe { &mut *self.inner.get() }.try_flush(&**self.state);
    }
}

/***** impl LocalAccess ***************************************************************************/

impl<'a> LocalAccess for &'a Local {
    type Reclaimer = Debra;

    #[inline]
    fn is_active(self) -> bool {
        self.guard_count.get() > 0
    }

    #[inline]
    fn set_active(self) {
        let count = self.guard_count.get();
        // this might THEORETICALLY overflow, but a check here adds 1-2 ns in
        // the fast path, which is not worth it
        self.guard_count.set(count + 1);

        if count == 0 {
            let inner = unsafe { &mut *self.inner.get() };
            inner.set_active(&**self.state);
        }
    }

    #[inline]
    fn set_inactive(self) {
        let count = self.guard_count.get();
        self.guard_count.set(count - 1);
        if count == 1 {
            let inner = unsafe { &*self.inner.get() };
            inner.set_inactive(&**self.state);
        } else if count == 0 {
            panic!("guard count overflow detected");
        }
    }

    #[inline]
    fn retire_record(self, record: Retired) {
        let inner = unsafe { &mut *self.inner.get() };
        inner.retire_record(record);
    }
}

/***** impl Default *******************************************************************************/

impl Default for Local {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/***** impl Drop **********************************************************************************/

impl Drop for Local {
    #[inline]
    fn drop(&mut self) {
        // remove thread entry from list and retire as last record
        let state = unsafe { ptr::read(&*self.state) };
        let entry = THREADS.remove(state);

        unsafe {
            let retired = Retired::new_unchecked(entry);
            let inner = &mut *self.inner.get();
            inner.retire_final_record(retired);
        }
    }
}
