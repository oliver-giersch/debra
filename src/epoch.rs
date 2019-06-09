//! Type safe epochs

use core::ops::{Add, Sub};
use core::sync::atomic::{AtomicUsize, Ordering};

const EPOCH_INCREMENT: usize = 2;
const QUIESCENT_BIT: usize = 0b1;

////////////////////////////////////////////////////////////////////////////////////////////////////
// AtomicEpoch
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
pub(crate) struct AtomicEpoch(AtomicUsize);

impl AtomicEpoch {
    #[inline]
    pub const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }

    #[inline]
    pub fn load(&self, order: Ordering) -> Epoch {
        Epoch(self.0.load(order))
    }

    #[inline]
    pub fn compare_and_swap(&self, current: Epoch, new: Epoch, order: Ordering) -> Epoch {
        Epoch(self.0.compare_and_swap(current.0, new.0, order))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Epoch
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A monotonically increasing epoch counter with wrapping overflow behaviour.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Epoch(usize);

impl Epoch {
    #[inline]
    pub fn increment(self) -> Self {
        Self(self.0.wrapping_add(EPOCH_INCREMENT))
    }

    #[inline]
    fn into_inner(self) -> usize {
        self.0
    }
}

impl Add<usize> for Epoch {
    type Output = Self;

    #[inline]
    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0.wrapping_add(rhs * EPOCH_INCREMENT))
    }
}

impl Sub<usize> for Epoch {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0.wrapping_sub(rhs * EPOCH_INCREMENT))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ThreadState
////////////////////////////////////////////////////////////////////////////////////////////////////

/// The concurrently accessible state of a thread.
#[derive(Debug)]
pub(crate) struct ThreadState(AtomicUsize);

impl ThreadState {
    #[inline]
    pub fn new(global_epoch: Epoch) -> Self {
        Self(AtomicUsize::new(global_epoch.into_inner() | QUIESCENT_BIT))
    }

    #[inline]
    pub fn is_same(&self, other: &Self) -> bool {
        self as *const Self == other as *const Self
    }

    #[inline]
    pub fn load_decompose(&self, order: Ordering) -> (Epoch, bool) {
        let state = self.0.load(order);
        (Epoch(state & !QUIESCENT_BIT), state & QUIESCENT_BIT == 0)
    }

    #[inline]
    pub fn store(&self, epoch: Epoch, state: State, order: Ordering) {
        match state {
            State::Active => self.0.store(epoch.0, order),
            State::Quiescent => self.0.store(epoch.0 | QUIESCENT_BIT, order),
        };
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// State
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum State {
    Active,
    Quiescent,
}
