//! DEBRA - Distributed Epoch Based Reclamation

#![feature(manually_drop_take)]
#![warn(missing_docs)]
#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use core::fmt;

pub use reclaim;
pub use reclaim::typenum;

pub use crate::local::Local;

/// A specialization of [`Atomic`][reclaim::Atomic] for the [`Debra`]
/// reclamation scheme.
pub type Atomic<T, N = U0> = reclaim::Atomic<T, Debra, N>;
/// A specialization of [`Owned`][reclaim::Owned] for the [`Debra`]
/// reclamation scheme.
pub type Owned<T, N = U0> = reclaim::Owned<T, Debra, N>;
/// A specialization of [`Shared`][reclaim::Shared] for the [`Debra`]
/// reclamation scheme.
pub type Shared<'g, T, N = U0> = reclaim::Shared<'g, T, Debra, N>;
/// A specialization of [`Unlinked`][reclaim::Unlinked] for the [`Debra`]
/// reclamation scheme.
pub type Unlinked<T, N = U0> = reclaim::Unlinked<T, Debra, N>;
/// A specialization of [`Unprotected`][reclaim::Unprotected] for the [`Debra`]
/// reclamation scheme.
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

/// Distributed epoch based reclamation.
#[derive(Copy, Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Debra;

impl fmt::Display for Debra {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "distributed epoch based reclamation")
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
