#[cfg(feature = "std")]
use conquer_once::spin::OnceCell;
#[cfg(not(feature = "std"))]
use conquer_once::OnceCell;

const DEFAULT_CHECK_THRESHOLD: u32 = 100;
const DEFAULT_ADVANCE_THRESHOLD: u32 = 100;

/// Global configuration for the reclamation scheme.
pub static CONFIG: OnceCell<Config> = OnceCell::new();

////////////////////////////////////////////////////////////////////////////////////////////////////
// Config
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A type containing configuration parameters for the DEBRA reclamation scheme.
#[derive(Copy, Clone, Debug)]
pub struct Config {
    check_threshold: u32,
    advance_threshold: u32,
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
        Self {
            check_threshold: DEFAULT_CHECK_THRESHOLD,
            advance_threshold: DEFAULT_ADVANCE_THRESHOLD,
        }
    }

    /// Creates a new [`Config`] with the given parameters.
    #[inline]
    pub fn with_params(check_threshold: u32, advance_threshold: u32) -> Self {
        assert!(check_threshold > 0, "the check threshold must be larger than 0");
        Self { check_threshold, advance_threshold }
    }

    #[inline]
    /// Returns the check threshold of the [`Config`].
    pub fn check_threshold(self) -> u32 {
        self.check_threshold
    }

    /// Returns the advance threshold of the [`Config`].
    #[inline]
    pub fn advance_threshold(self) -> u32 {
        self.advance_threshold
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ConfigBuilder
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A builder type for [`Config`] instances.
#[derive(Copy, Clone, Debug, Default)]
pub struct ConfigBuilder {
    check_threshold: Option<u32>,
    advance_threshold: Option<u32>,
}

impl ConfigBuilder {
    /// Creates a new [`ConfigBuilder`].
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the check threshold.
    #[inline]
    pub fn check_threshold(mut self, check_threshold: u32) -> Self {
        self.check_threshold = Some(check_threshold);
        self
    }

    /// Sets the advance threshold.
    #[inline]
    pub fn advance_threshold(mut self, advance_threshold: u32) -> Self {
        self.advance_threshold = Some(advance_threshold);
        self
    }

    /// Consumes the builder and creates a new [`Config`] instance with the
    /// configured parameters or their default values, if they were not set.
    #[inline]
    pub fn build(self) -> Config {
        Config {
            check_threshold: self.check_threshold.unwrap_or(DEFAULT_CHECK_THRESHOLD),
            advance_threshold: self.advance_threshold.unwrap_or(DEFAULT_ADVANCE_THRESHOLD),
        }
    }
}
