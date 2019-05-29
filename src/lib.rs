#![warn(missing_docs)]
use core::sync::atomic::Ordering;

pub use reclaim;
pub use reclaim::typenum;

pub type Atomic<T, N = U0> = reclaim::Atomic<T, Debra, N>;
pub type Owned<T, N = U0> = reclaim::Owned<T, Debra, N>;
pub type Shared<'g, T, N = U0> = reclaim::Shared<'g, T, Debra, N>;
pub type Unlinked<T, N = U0> = reclaim::Unlinked<T, Debra, N>;
pub type Unprotected<T, N = U0> = reclaim::Unprotected<T, Debra, N>;

pub use crate::thread::Local;

use typenum::{Unsigned, U0};
use reclaim::{AcquireResult, LocalReclaim, Marked, MarkedPtr, Reclaim, Protect};

mod bag;
mod epoch;
mod global;
mod local;
mod thread;
mod threads;
mod retired;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debra
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub struct Debra;

unsafe impl Reclaim for Debra {
    unsafe fn retire<T: 'static, N: Unsigned>(unlinked: Unlinked<T, N>) {
        unimplemented!()
    }

    unsafe fn retire_unchecked<T, N: Unsigned>(unlinked: Unlinked<T, N>) {
        unimplemented!()
    }
}

unsafe impl LocalReclaim for Debra {
    type Local = Local;
    type RecordHeader = ();

    unsafe fn retire_local<T: 'static, N: Unsigned>(local: &Self::Local, unlinked: Unlinked<T, N>) {
        unimplemented!()
    }

    unsafe fn retire_local_unchecked<T, N: Unsigned>(local: &Self::Local, unlinked: Unlinked<T, N>) {
        unimplemented!()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guarded
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Guarded<T, N: Unsigned>(T, N);

unsafe impl Protect for Guarded<T, N> {
    type Item = ();
    type Reclaimer = ();
    type MarkBits = ();

    fn marked(&self) -> Marked<Shared<T, N>> {
        unimplemented!()
    }

    fn acquire(&mut self, atomic: &Atomic<T, N>, order: Ordering) -> Marked<Shared<T, N>> {
        unimplemented!()
    }

    fn acquire_if_equal(&mut self, atomic: &Atomic<T, N>, expected: MarkedPtr<T, N>, order: Ordering) -> AcquireResult<T, Debra, N> {
        unimplemented!()
    }

    fn release(&mut self) {
        unimplemented!()
    }
}

