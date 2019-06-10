//! Caching and deferred deletion of type-erased retired records.

use core::fmt;
use core::mem;
use core::ptr::NonNull;

type Record<T> = reclaim::Record<T, crate::Debra>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retired
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A type-erased fat pointer to a retired record.
pub struct Retired(NonNull<dyn Any + 'static>);

impl Retired {
    /// # Safety
    ///
    /// ...
    #[inline]
    pub unsafe fn new_unchecked<'a, T: 'a>(record: NonNull<T>) -> Self {
        let any: NonNull<dyn Any + 'a> = Record::from_raw_non_null(record);
        let any: NonNull<dyn Any + 'static> = mem::transmute(any);

        Self(any)
    }

    /// Returns the memory address of the retired record.
    #[inline]
    pub fn address(&self) -> usize {
        self.0.as_ptr() as *const _ as *const () as usize
    }

    /// Reclaims the retired record.
    #[inline]
    pub unsafe fn reclaim(&mut self) {
        mem::drop(Box::from_raw(self.0.as_ptr()));
    }
}

impl fmt::Debug for Retired {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Retired").field("address", &(self.address() as *const ())).finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Any (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

trait Any {}
impl<T> Any for T {}
