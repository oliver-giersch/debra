use core::sync::atomic::Ordering;

use reclaim::prelude::*;
use reclaim::{AcquireResult, MarkedNonNull, MarkedPtr};

use crate::local::{Local, LocalAccess};
use crate::typenum::Unsigned;
use crate::{Atomic, Debra, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guarded
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Guarded<T, N: Unsigned, L: LocalAccess> {
    marked: Marked<MarkedNonNull<T, N>>,
    local_access: L,
}

impl<T, N: Unsigned, L: LocalAccess> Guarded<T, N, L> {
    /// Creates a new [`Guarded`] with the given `local_access`.
    #[inline]
    pub fn with_local_access(local_access: L) -> Self {
        Self { marked: Null(0), local_access }
    }
}

impl<'a, T, N: Unsigned> Guarded<T, N, &'a Local> {
    /// Creates a new [`Guarded`] with the given reference to thread local
    /// [`Local`] state.
    #[inline]
    pub fn new(local_access: &'a Local) -> Self {
        Self::with_local_access(local_access)
    }
}

impl<T, N: Unsigned, L: LocalAccess> Clone for Guarded<T, N, L> {
    #[inline]
    fn clone(&self) -> Self {
        if self.marked.is_value() {
            self.local_access.set_active();
        }

        Self { marked: self.marked, local_access: self.local_access }
    }
}

unsafe impl<T, N: Unsigned, L: LocalAccess> Protect for Guarded<T, N, L> {
    type Item = T;
    type Reclaimer = Debra;
    type MarkBits = N;

    #[inline]
    fn marked(&self) -> Marked<Shared<T, N>> {
        self.marked.map(|ptr| unsafe { Shared::from_marked_non_null(ptr) })
    }

    #[inline]
    fn acquire(&mut self, atomic: &Atomic<T, N>, order: Ordering) -> Marked<Shared<T, N>> {
        if self.marked.is_null() {
            self.local_access.set_active();
        }

        self.marked = MarkedNonNull::new(atomic.load_raw(order));
        self.marked()
    }

    #[inline]
    fn acquire_if_equal(
        &mut self,
        _atomic: &Atomic<T, N>,
        _expected: MarkedPtr<T, N>,
        _order: Ordering,
    ) -> AcquireResult<T, Debra, N> {
        unimplemented!()
    }

    #[inline]
    fn release(&mut self) {
        if !self.marked.is_null() {
            self.local_access.set_inactive();
        }

        self.marked = Null(0);
    }
}
