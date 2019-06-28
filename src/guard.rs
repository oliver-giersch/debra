use core::sync::atomic::Ordering;

use debra_common::{reclaim, LocalAccess};
use reclaim::prelude::*;
use reclaim::{AcquireResult, MarkedPtr, NotEqualError};

use crate::local::Local;
use crate::typenum::Unsigned;
use crate::{Atomic, Debra, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Guard<L: LocalAccess> {
    local_access: L,
}

impl<'a> Guard<&'a Local> {
    /// Creates a new [`Guarded`] with the given reference to thread local
    /// [`Local`] state.
    #[inline]
    pub fn new(local_access: &'a Local) -> Self {
        Self::with_local_access(local_access)
    }
}

impl<L: LocalAccess> Guard<L> {
    /// Creates a new [`Guarded`] with the given `local_access`.
    #[inline]
    pub fn with_local_access(local_access: L) -> Self {
        local_access.set_active();
        Self { local_access }
    }
}

impl<L: LocalAccess> Clone for Guard<L> {
    #[inline]
    fn clone(&self) -> Self {
        self.local_access.set_active();
        Self { local_access: self.local_access }
    }
}

unsafe impl<L: LocalAccess<Reclaimer = Debra>> Protect for Guard<L> {
    type Reclaimer = Debra;

    #[inline]
    fn release(&mut self) {}

    #[inline]
    fn protect<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, N>,
        order: Ordering,
    ) -> Marked<Shared<T, N>> {
        unsafe { Marked::from_marked_ptr(atomic.load_raw(order)) }
    }

    #[inline]
    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> AcquireResult<T, Self::Reclaimer, N> {
        match atomic.load_raw(order) {
            ptr if ptr == expected => unsafe { Ok(Marked::from_marked_ptr(ptr)) },
            _ => Err(NotEqualError),
        }
    }
}

unsafe impl<L: LocalAccess<Reclaimer = Debra>> ProtectRegion for Guard<L> {}
