//! Thread local variables and access abstractions for *std* environments.

use std::sync::RwLock;

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
        Self::with_local_access(DefaultAccess)
    }
}

/***** impl Default *******************************************************************************/

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Config
////////////////////////////////////////////////////////////////////////////////////////////////////

const DEFAULT_CHECK_THRESHOLD: u32 = 100;
const DEFAULT_ADVANCE_THRESHOLD: u32 = 100;
const DEFAULT_BAG_SIZE: usize = 256;

pub struct GlobalConfig(RwLock<Config>);

impl GlobalConfig {
    pub fn configure(&self, check_threshold: u32, advance_threshold: u32, bag_size: usize) {
        let mut lock = self.0.write().unwrap();
        lock.check_threshold = check_threshold;
        lock.advance_threshold = advance_threshold;
        lock.bag_size = bag_size;
    }

    pub fn try_read_or_default(&self) -> Config {
        match self.0.try_read() {
            Ok(lock) => Config {
                check_threshold: lock.check_threshold,
                advance_threshold: lock.advance_threshold,
                bag_size: lock.bag_size,
            },
            Err(_) => Config {
                check_threshold: DEFAULT_CHECK_THRESHOLD,
                advance_threshold: DEFAULT_ADVANCE_THRESHOLD,
                bag_size: DEFAULT_BAG_SIZE,
            },
        }
    }
}

pub struct Config {
    check_threshold: u32,
    advance_threshold: u32,
    bag_size: usize,
}

impl Config {
    pub fn check_threshold(&self) -> u32 {
        self.check_threshold
    }

    pub fn advance_threshold(&self) -> u32 {
        self.advance_threshold
    }

    pub fn bag_size(&self) -> usize {
        self.bag_size
    }
}
