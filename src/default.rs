//! Thread local variables and access abstractions for *std* environments.

use std::marker::PhantomData;

use debra_common::reclaim;
use debra_common::LocalAccess;
use reclaim::{GlobalReclaim, Reclaim};

use crate::guard::Guard;
use crate::local::Local;
use crate::typenum::Unsigned;
use crate::{Debra, Retired, Unlinked};

thread_local!(static LOCAL: Local = Local::new());

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debra
////////////////////////////////////////////////////////////////////////////////////////////////////

/***** impl inherent ******************************************************************************/

impl Debra {
    /// Returns `true` if the current thread is active, i.e. has an at least one
    /// [`Guard`] in some scope.
    #[inline]
    pub fn is_thread_active() -> bool {
        LOCAL.with(|local| local.is_active())
    }
}

/***** impl GlobalReclaim *************************************************************************/

unsafe impl GlobalReclaim for Debra {
    type Guard = Guard<DefaultAccess>;

    #[inline]
    fn try_flush() {
        LOCAL.with(|local| local.try_flush());
    }

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
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

/***** impl inherent ******************************************************************************/

impl Guard<DefaultAccess> {
    #[inline]
    pub fn new() -> Self {
        Self::with_local_access(DefaultAccess::default())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// DefaultAccess
////////////////////////////////////////////////////////////////////////////////////////////////////

// An alternative implementation using a raw pointer to the thread local state
// was tested, which avoided some runtime check done by the std thread local
// implementation.
// When bench-marked, this alternative was surprisingly found out to be slower
// and hence discarded
#[derive(Copy, Clone, Debug, Default)]
pub struct DefaultAccess(PhantomData<*mut ()>);

/***** impl LocalAccess ***************************************************************************/

impl LocalAccess for DefaultAccess {
    type Reclaimer = Debra;

    #[inline]
    fn is_active(self) -> bool {
        LOCAL.with(|local| local.is_active())
    }

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
