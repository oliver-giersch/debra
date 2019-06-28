//! Thread local variables and access abstractions for *std* environments.

use debra_common::reclaim;
use debra_common::LocalAccess;
use reclaim::{GlobalReclaim, Reclaim};

use crate::guard::Guard;
use crate::local::Local;
use crate::typenum::Unsigned;
use crate::{Debra, Retired, Unlinked};

thread_local!(static LOCAL: Local = Local::new());

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Reclaim
////////////////////////////////////////////////////////////////////////////////////////////////////

unsafe impl GlobalReclaim for Debra {
    type Guard = Guard<DefaultAccess>;

    #[inline]
    unsafe fn retire<T: 'static, N: Unsigned>(unlinked: Unlinked<T, N>) {
        LOCAL.with(move |local| Self::retire_local(local, unlinked));
    }

    #[inline]
    unsafe fn retire_unchecked<T, N: Unsigned>(unlinked: Unlinked<T, N>) {
        LOCAL.with(move |local| Self::retire_local_unchecked(local, unlinked));
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guarded
////////////////////////////////////////////////////////////////////////////////////////////////////

impl Guard<DefaultAccess> {
    #[inline]
    pub fn new() -> Self {
        Self::with_local_access(DefaultAccess)
    }
}

impl Default for Guard<DefaultAccess> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// DefaultAccess
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, Default)]
pub struct DefaultAccess;

impl LocalAccess for DefaultAccess {
    type Reclaimer = Debra;

    #[inline]
    fn set_active(self) {
        LOCAL.with(|local| local.set_active());
    }

    #[inline]
    fn set_inactive(self) {
        LOCAL.with(|local| local.set_inactive());
    }

    #[inline]
    fn retire_record(self, record: Retired) {
        LOCAL.with(move |local| local.retire_record(record));
    }
}
