//! Type-erased caching of retired records

use core::fmt;
use core::ptr::NonNull;

use arrayvec::ArrayVec;

////////////////////////////////////////////////////////////////////////////////
// BagQueue
////////////////////////////////////////////////////////////////////////////////

/// A LIFO queue of [`RetiredBag`]s.
#[derive(Debug)]
pub(crate) struct BagQueue {
    head: Option<Box<RetiredBag>>,
}

////////////////////////////////////////////////////////////////////////////////
// RetiredBag
////////////////////////////////////////////////////////////////////////////////

const DEFAULT_BAG_SIZE: usize = 256;

#[derive(Debug)]
pub(crate) struct RetiredBag {
    next: Option<Box<RetiredBag>>,
    retired: ArrayVec<[Retired; DEFAULT_BAG_SIZE]>,
}

impl RetiredBag {
    #[inline]
    pub fn new() -> Self {
        Self { next: None, retired: ArrayVec::default() }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Retired
////////////////////////////////////////////////////////////////////////////////

type Record<T> = reclaim::Record<T, crate::Debra>;

pub(crate) struct Retired(Box<dyn Any + 'static>);

impl Retired {
    #[inline]
    pub fn address(&self) -> usize {
        &*self.0 as *const _ as *const () as usize
    }
}

impl fmt::Debug for Retired {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Retired").field("address", &(self.address() as *const ())).finish()
    }
}

////////////////////////////////////////////////////////////////////////////////
// Any (trait)
////////////////////////////////////////////////////////////////////////////////

trait Any {}
impl<T> Any for T {}