#![warn(missing_docs)]

pub use reclaim;
pub use reclaim::typenum;

pub type Atomic<T, N = U0> = reclaim::Atomic<T, Debra, N>;
pub type Owned<T, N = U0> = reclaim::Owned<T, Debra, N>;
pub type Shared<'g, T, N = U0> = reclaim::Shared<'g, T, Debra, N>;
pub type Unlinked<T, N = U0> = reclaim::Unlinked<T, Debra, N>;
pub type Unprotected<T, N = U0> = reclaim::Unprotected<T, Debra, N>;

use typenum::{Unsigned, U0};
use reclaim::{LocalReclaim, Reclaim};

mod epoch;
mod thread;
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
    type Local = ();
    type RecordHeader = ();

    unsafe fn retire_local<T: 'static, N: Unsigned>(local: &Self::Local, unlinked: Unlinked<T, N>) {
        unimplemented!()
    }

    unsafe fn retire_local_unchecked<T, N: Unsigned>(local: &Self::Local, unlinked: Unlinked<T, N>) {
        unimplemented!()
    }
}