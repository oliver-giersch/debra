use core::cell::UnsafeCell;
use core::sync::atomic::{
    AtomicU8,
    Ordering::{Acquire, Release},
};

const DEFAULT_CHECK_THRESHOLD: u32 = 100;
const DEFAULT_ADVANCE_THRESHOLD: u32 = 100;

const UNINIT: u8 = 0;
const BUSY: u8 = 1;
const READY: u8 = 2;

////////////////////////////////////////////////////////////////////////////////////////////////////
// GlobalConfig
////////////////////////////////////////////////////////////////////////////////////////////////////

/// One-time global lock-free configuration for the DEBRA reclamation scheme.
#[derive(Debug)]
pub struct GlobalConfig {
    init_state: AtomicU8,
    config: UnsafeCell<Config>,
}

/***** impl Sync **********************************************************************************/

unsafe impl Sync for GlobalConfig {}

/***** impl inherent ******************************************************************************/

impl GlobalConfig {
    /// Creates a new uninitialized [`GlobalConfig`].
    #[inline]
    pub const fn new() -> Self {
        Self { init_state: AtomicU8::new(UNINIT), config: UnsafeCell::new(Config::new()) }
    }

    /// Initializes the [`GlobalConfig`] with the given `config`, but only once.
    #[inline]
    pub fn init_once(&self, config: Config) {
        if UNINIT == self.init_state.compare_and_swap(UNINIT, BUSY, Acquire) {
            let inner = unsafe { &mut *self.config.get() };
            *inner = config;
            self.init_state.store(READY, Release);
        }
    }

    /// Reads the initialized [`Config`] or returns the default configuration,
    /// if the [`GlobalConfig`] is either not or currently in the process of
    /// being initialized.
    #[inline]
    pub(crate) fn read_config_or_default(&self) -> Config {
        if self.init_state.load(Acquire) == READY {
            unsafe { *&*self.config.get() }
        } else {
            Config::default()
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Config
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A type containing configuration parameters for the DEBRA reclamation scheme.
#[derive(Copy, Clone, Debug)]
pub struct Config {
    check_thresh: u32,
    advance_thresh: u32,
}

/***** impl Default *******************************************************************************/

impl Default for Config {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/***** impl inherent ******************************************************************************/

impl Config {
    /// Creates a new default [`Config`].
    #[inline]
    pub const fn new() -> Self {
        Self { check_thresh: DEFAULT_CHECK_THRESHOLD, advance_thresh: DEFAULT_ADVANCE_THRESHOLD }
    }

    #[inline]
    /// Returns the check threshold of the [`Config`].    
    pub fn check_threshold(&self) -> u32 {
        self.check_thresh
    }

    /// Returns the advance threshold of the [`Config`].
    #[inline]
    pub fn advance_threshold(&self) -> u32 {
        self.advance_thresh
    }
}
