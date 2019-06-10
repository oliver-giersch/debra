use core::sync::atomic::Ordering;

use reclaim::prelude::*;
use reclaim::{AcquireResult, MarkedNonNull, MarkedPtr};

use crate::local::LocalAccess;
use crate::typenum::Unsigned;
use crate::{Atomic, Debra, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guarded
////////////////////////////////////////////////////////////////////////////////////////////////////

pub(crate) struct Guarded<T, N: Unsigned, L: LocalAccess> {
    marked: Marked<MarkedNonNull<T, N>>,
    local_access: L,
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
        atomic: &Atomic<T, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> AcquireResult<T, Debra, N> {
        unimplemented!()
    }

    #[inline]
    fn release(&mut self) {
        unimplemented!()
    }
}
