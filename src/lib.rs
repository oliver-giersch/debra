#![feature(manually_drop_take)]
#![warn(missing_docs)]
use core::sync::atomic::Ordering;

pub use reclaim;
pub use reclaim::typenum;

pub type Atomic<T, N = U0> = reclaim::Atomic<T, Debra, N>;
pub type Owned<T, N = U0> = reclaim::Owned<T, Debra, N>;
pub type Shared<'g, T, N = U0> = reclaim::Shared<'g, T, Debra, N>;
pub type Unlinked<T, N = U0> = reclaim::Unlinked<T, Debra, N>;
pub type Unprotected<T, N = U0> = reclaim::Unprotected<T, Debra, N>;

pub use crate::local::Local;

use crate::retired::Retired;
use reclaim::prelude::*;
use reclaim::{AcquireResult, MarkedPtr};
use typenum::{Unsigned, U0};

mod abandoned;
mod epoch;
mod global;
mod list;
mod local;
mod retired;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debra
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Debra;

// TODO: Move to default module
unsafe impl Reclaim for Debra {
    #[inline]
    unsafe fn retire<T: 'static, N: Unsigned>(unlinked: Unlinked<T, N>) {
        unimplemented!()
    }

    #[inline]
    unsafe fn retire_unchecked<T, N: Unsigned>(unlinked: Unlinked<T, N>) {
        unimplemented!()
    }
}

unsafe impl LocalReclaim for Debra {
    type Local = Local;
    type RecordHeader = ();

    #[inline]
    unsafe fn retire_local<T: 'static, N: Unsigned>(local: &Self::Local, unlinked: Unlinked<T, N>) {
        Self::retire_local_unchecked(local, unlinked);
    }

    #[inline]
    unsafe fn retire_local_unchecked<T, N: Unsigned>(
        local: &Self::Local,
        unlinked: Unlinked<T, N>,
    ) {
        let unmarked = unlinked.into_marked_non_null().decompose_non_null();
        local.retire_record(Retired::new_unchecked(unmarked));
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guarded
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Guarded<T, N: Unsigned>(T, N);

impl<T, N: Unsigned> Clone for Guarded<T, N> {
    #[inline]
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

unsafe impl<T, N: Unsigned> Protect for Guarded<T, N> {
    type Item = T;
    type Reclaimer = Debra;
    type MarkBits = N;

    fn marked(&self) -> Marked<Shared<T, N>> {
        unimplemented!()
    }

    fn acquire(&mut self, atomic: &Atomic<T, N>, order: Ordering) -> Marked<Shared<T, N>> {
        unimplemented!()
    }

    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<T, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> AcquireResult<T, Debra, N> {
        unimplemented!()
    }

    fn release(&mut self) {
        unimplemented!()
    }
}
