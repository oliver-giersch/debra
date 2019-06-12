//! Type safe epochs

use core::ops::{Add, Sub};
use core::sync::atomic::{AtomicUsize, Ordering};
use std::io::SeekFrom::Start;

const EPOCH_INCREMENT: usize = 2;
const QUIESCENT_BIT: usize = 0b1;

////////////////////////////////////////////////////////////////////////////////////////////////////
// AtomicEpoch
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A concurrently accessible [`Epoch`].
pub(crate) struct AtomicEpoch(AtomicUsize);

impl AtomicEpoch {
    /// Creates a new [`AtomicEpoch`] starting at zero.
    #[inline]
    pub const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }

    /// Loads the epoch with the specified `order`.
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
    /// Returns the [`PossibleAge`] of the epoch relative to `global_epoch`.
    ///
    /// Since the global epoch is explicitly allowed to wrap around, it is not
    /// possible to unambiguously determine the relative age of an epoch.
    /// However, since epochs are monotonically increasing it is certain that
    /// any previously observed epoch must be older of equal than the global
    /// epoch.
    /// Consequently, it is possible to determine if an epoch **could** be
    /// within the critical range of two epochs, during which reclamation of
    /// records **must** be avoided, and is in order to be conservative.
    #[inline]
    pub fn relative_age(self, global_epoch: Epoch) -> Result<PossibleAge, Undetermined> {
        match global_epoch.0.wrapping_sub(self.0) {
            0 => Ok(PossibleAge::SameEpoch),
            1 => Ok(PossibleAge::OneEpoch),
            2 => Ok(PossibleAge::TwoEpochs),
            _ => Err(Undetermined),
        }
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
// PossibleAge
////////////////////////////////////////////////////////////////////////////////////////////////////

/// The possible age of an epoch in relation to global epoch within a two-epoch
/// range.
///
/// See [`relative_age`][Epoch::relative_age] for more details.
#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum PossibleAge {
    SameEpoch,
    OneEpoch,
    TwoEpochs,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Undetermined
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Undetermined;

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
    pub fn load_decompose(&self, order: Ordering) -> (Epoch, State) {
        let state = self.0.load(order);
        (Epoch(state & !QUIESCENT_BIT), State::from(state & QUIESCENT_BIT == 0))
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

impl From<bool> for State {
    #[inline]
    fn from(state: bool) -> Self {
        match state {
            true => State::Active,
            false => State::Quiescent,
        }
    }
}
