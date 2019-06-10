//! TODO: Docs...

#![feature(manually_drop_take)]
#![warn(missing_docs)]
#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub use reclaim;
pub use reclaim::typenum;

pub use crate::local::Local;

pub type Atomic<T, N = U0> = reclaim::Atomic<T, Debra, N>;
pub type Owned<T, N = U0> = reclaim::Owned<T, Debra, N>;
pub type Shared<'g, T, N = U0> = reclaim::Shared<'g, T, Debra, N>;
pub type Unlinked<T, N = U0> = reclaim::Unlinked<T, Debra, N>;
pub type Unprotected<T, N = U0> = reclaim::Unprotected<T, Debra, N>;

// FIXME: only if no_std
pub type LocalGuarded<'a, T, N> = crate::guarded::Guarded<T, N, &'a Local>;

use reclaim::prelude::*;
use typenum::{Unsigned, U0};

use crate::local::LocalAccess;
use crate::retired::Retired;

#[cfg(feature = "std")]
mod default;

mod abandoned;
mod bag;
mod epoch;
mod global;
mod guarded;
mod list;
mod local;
mod retired;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debra
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Debra;

/*
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
}*/

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
