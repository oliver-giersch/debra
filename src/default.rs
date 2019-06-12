//! Thread local variables and access abstractions for *std* environments.

use reclaim::{LocalReclaim, Reclaim};

use crate::guarded::Guarded;
use crate::local::{Local, LocalAccess};
use crate::retired::Retired;
use crate::typenum::Unsigned;
use crate::{Debra, Unlinked};

thread_local!(static LOCAL: Local = Local::new());

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Reclaim
////////////////////////////////////////////////////////////////////////////////////////////////////

unsafe impl Reclaim for Debra {
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

impl<T, N: Unsigned> Guarded<T, N, DefaultAccess> {
    #[inline]
    pub fn new() -> Self {
        Self::with_local_access(DefaultAccess)
    }
}

impl<T, N: Unsigned> Default for Guarded<T, N, DefaultAccess> {
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
