//! Type safe epochs

use core::sync::atomic::{AtomicUsize, Ordering};

const EPOCH_INCREMENT: usize = 2;
const QUIESCENT_BIT: usize = 0b1;

/// TODO: Doc...
pub(crate) struct AtomicEpoch(AtomicUsize);

impl AtomicEpoch {
    #[inline]
    pub const fn new() -> Self {
        Self(AtomicUsize::new(EPOCH_INCREMENT))
    }

    #[inline]
    pub fn load(&self, order: Ordering) -> Epoch {
        Epoch(self.0.load(order))
    }

    #[inline]
    pub fn store(&self, epoch: Epoch, order: Ordering) {
        self.0.store(epoch.0, order);
    }
}

/// A representation for a monotonically increasing epoch counter with wrapping
/// behaviour.
pub(crate) struct Epoch(usize);

impl Epoch {
    #[inline]
    pub fn increment(self) -> Self {
        Self(self.0.wrapping_add(EPOCH_INCREMENT))
    }
}

/// TODO: Doc...
pub(crate) struct ThreadEpoch(AtomicUsize);

impl ThreadEpoch {
    /// TODO: Doc...
    #[inline]
    pub fn load_decompose(&self, order: Ordering) -> (Epoch, bool) {
        unimplemented!()
    }
}

// FIXME: better name: pub(crate) struct ThreadState(pub Epoch, pub bool);