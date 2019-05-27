//! Type safe epochs

use core::sync::atomic::Ordering;

const EPOCH_INCREMENT: usize = 2;
const QUIESCENT_BIT: usize = 0b1;

/// TODO: Doc...
pub(crate) struct AtomicEpoch;

/// A representation for a monotonically increasing epoch counter with wrapping
/// behaviour.
pub(crate) struct Epoch(usize);

/// TODO: Doc...
pub(crate) struct ThreadEpoch;

impl ThreadEpoch {
    /// TODO: Doc...
    #[inline]
    pub fn load_decompose(&self, order: Ordering) -> (Epoch, bool) {
        unimplemented!()
    }
}