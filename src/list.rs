//! A concurrent lock-free list that is ordered by the (heap) addresses of its
//! entries and does not deallocate memory of entries removed during its
//! lifetime.

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use core::marker::PhantomData;
use core::mem;
use core::ops::Deref;
use core::ptr::{self, NonNull};
use core::sync::atomic::Ordering::{self, Acquire, Relaxed, Release};

use crate::reclaim::align::CacheAligned;
use crate::reclaim::prelude::*;
use crate::reclaim::typenum::U1;
use crate::reclaim::{MarkedNonNull, MarkedPtr};

type AtomicMarkedPtr<T> = crate::reclaim::AtomicMarkedPtr<T, U1>;

const REMOVE_TAG: usize = 0b1;

////////////////////////////////////////////////////////////////////////////////////////////////////
// List
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A concurrent lock-free list with restricted permissions for removal of
/// entries.
///
/// Each entry in the queue is associated to an owner, represented by a
/// [`SetEntry`]. Only this owner can remove the entry again from the queue,
/// which may be located at an arbitrary position in the queue.
#[derive(Debug)]
pub(crate) struct List<T> {
    head: AtomicMarkedPtr<Node<T>>,
}

/***** impl inherent ******************************************************************************/

impl<T> List<T> {
    /// Creates a new empty [`List`].
    pub const fn new() -> Self {
        Self { head: AtomicMarkedPtr::null() }
    }

    /// Inserts the given `entry` and returns an owned [`SetEntry`] token.
    ///
    /// The returned token is the only way, by which an entry can be removed
    /// from the list again and also acts like a shared reference to the entry.
    #[inline]
    pub fn insert(&self, entry: T) -> ListEntry<T> {
        let entry = Box::leak(Box::new(Node::new(entry)));
        loop {
            let head = self.head.load(Acquire);
            entry.next().store(head, Relaxed);

            if self
                .head
                .compare_exchange_weak(head, MarkedPtr::new(entry), Release, Relaxed)
                .is_ok()
            {
                return ListEntry(NonNull::from(entry), PhantomData);
            }
        }
    }

    /// Removes the given `entry` from the list and returns a pointer to the
    /// entry's heap address, which can be transformed back into a [`Box`].
    ///
    /// It is in the responsibility of the caller to not deallocate the entry
    /// too soon, since other threads could still be accessing the removed
    /// value.
    ///
    /// # Panics
    ///
    /// Panics if the given `entry` belongs to a different list.
    #[inline]
    pub fn remove(&self, entry: ListEntry<T>) -> NonNull<Node<T>> {
        let entry = entry.into_inner();
        loop {
            let pos = self
                .iter_inner(None)
                .find(|pos| pos.curr == entry)
                .expect("given `entry` does not exist in this set");

            let prev = unsafe { pos.prev.as_ref() };
            let curr = unsafe { pos.curr.as_ref() };
            let next = MarkedPtr::new(pos.next.unwrap_ptr());
            let next_marked = MarkedPtr::compose(pos.next.unwrap_ptr(), REMOVE_TAG);

            // (LIS:2) this `Acquire` CAS synchronizes-with the `Release` CAS (LIS:1) and (LIS:3)
            if curr.next.compare_exchange(next, next_marked, Acquire, Relaxed).is_err() {
                continue;
            }

            // (LIS:3) this `Release` CAS synchronizes-with the `Acquire` loads (INN:3), (INN:4),
            // (LIS:4), (LIS:5) and the `Acquire` CAS (LIS:2)
            if prev.compare_exchange(MarkedPtr::from(curr), next, Release, Relaxed).is_err() {
                self.repeat_remove(entry);
            }

            return entry;
        }
    }

    /// Returns an iterator over the list.
    #[inline]
    pub fn iter(&self) -> Iter<T> {
        Iter::new(self, &self.head)
    }

    /// Loops until a marked node containing `entry` is successfully removed.
    #[inline]
    fn repeat_remove(&self, entry: NonNull<Node<T>>) {
        loop {
            let pos = self
                .iter_inner(Some(entry))
                .find(|pos| pos.curr == entry)
                .unwrap_or_else(|| unreachable!());

            let prev = unsafe { pos.prev.as_ref() };
            let curr = MarkedPtr::new(pos.curr.as_ptr());
            let next = MarkedPtr::new(pos.next.unwrap_ptr());

            // same as (LIS:3)
            if prev.compare_exchange(curr, next, Release, Relaxed).is_ok() {
                return;
            }
        }
    }

    /// Returns an internal iterator over the list.
    #[inline]
    fn iter_inner(&self, ignore: Option<NonNull<Node<T>>>) -> IterInner<T> {
        IterInner { head: &self.head, prev: NonNull::from(&self.head), ignore }
    }
}

/***** impl Drop **********************************************************************************/

impl<T> Drop for List<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let mut node = self.head.load(Relaxed).as_ref();
            while let Some(curr) = node {
                node = curr.next().load(Relaxed).as_ref();
                mem::drop(Box::from_raw(curr as *const _ as *mut Node<T>));
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ListEntry
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A token representing ownership of an entry in a [`List`]
#[derive(Debug)]
#[must_use]
pub(crate) struct ListEntry<'a, T>(NonNull<Node<T>>, PhantomData<&'a List<T>>);

/***** impl inherent ******************************************************************************/

impl<T> ListEntry<'_, T> {
    #[inline]
    fn into_inner(self) -> NonNull<Node<T>> {
        let inner = self.0;
        mem::forget(self);
        inner
    }
}

/***** impl Deref *********************************************************************************/

impl<T> Deref for ListEntry<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let node = unsafe { &*self.0.as_ptr() };
        &*node.elem
    }
}

/***** impl Drop **********************************************************************************/

impl<T> Drop for ListEntry<'_, T> {
    #[inline]
    fn drop(&mut self) {
        panic!("set entries must be used to remove their associated entry");
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A node containing an entry of a [`List`]
#[derive(Debug, Default)]
pub(crate) struct Node<T> {
    elem: CacheAligned<T>,
    next: CacheAligned<AtomicMarkedPtr<Node<T>>>,
}

/***** impl inherent ******************************************************************************/

impl<T> Node<T> {
    /// Returns a reference to the node's element.
    #[inline]
    fn elem(&self) -> &T {
        &*self.elem
    }

    /// Returns a reference to the node's `next` pointer.
    #[inline]
    fn next(&self) -> &AtomicMarkedPtr<Node<T>> {
        &*self.next
    }

    /// Creates a new [`Node`].
    #[inline]
    fn new(elem: T) -> Self {
        Self { elem: CacheAligned(elem), next: CacheAligned(AtomicMarkedPtr::null()) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Iter
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An iterator over a [`List`].
#[derive(Debug)]
pub(crate) struct Iter<'a, T>(IterInner<'a, T>);

/***** impl Iterator ******************************************************************************/

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|IterPos { curr, .. }| unsafe { &*curr.as_ptr() }.elem())
    }
}

/***** impl inherent ******************************************************************************/

impl<'a, T> Iter<'a, T> {
    /// Creates a new iterator for the given `list` that starts at the given
    /// list position.
    #[inline]
    pub fn new(list: &'a List<T>, start: &AtomicMarkedPtr<Node<T>>) -> Self {
        Self(IterInner { head: &list.head, prev: NonNull::from(start), ignore: None })
    }

    /// Loads the entry and its tag at the current position of the iterator.
    ///
    /// # Errors
    ///
    /// Returns an error if a node is loaded whose predecessor is already marked
    /// for removal.
    #[inline]
    pub fn load_current(&mut self, order: Ordering) -> Result<Option<&'a T>, IterError> {
        let (curr, tag) = unsafe { self.0.prev.as_ref().load(order).decompose_ref() };
        if tag == REMOVE_TAG {
            Err(IterError::Retry)
        } else {
            Ok(curr.map(|node| node.elem()))
        }
    }

    /// Loads and dereferences the current value of the [`List`]'s head.
    #[inline]
    pub fn load_head(&self, order: Ordering) -> Option<&'a T> {
        unsafe { self.0.head.load(order).as_ref().map(|node| node.elem()) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IterError
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An error that can occur during the iteration of a [`List`].
pub(crate) enum IterError {
    /// The iterators current element has been marked for removal and the
    /// iterator has to restart.
    Retry,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IterInner
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A module internal iterator over a [`List`].
#[derive(Debug)]
struct IterInner<'a, T> {
    head: &'a AtomicMarkedPtr<Node<T>>,
    prev: NonNull<AtomicMarkedPtr<Node<T>>>,
    ignore: Option<NonNull<Node<T>>>,
}

/***** impl Iterator ******************************************************************************/

impl<T> Iterator for IterInner<'_, T> {
    type Item = IterPos<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // (LIS:4) this `Acquire` load synchronizes-with the the `Release` CAS (LIS:1) and (LIS:3)
        while let Value(curr) = unsafe { MarkedNonNull::new(self.prev.as_ref().load(Acquire)) } {
            let (curr, curr_tag) = unsafe { curr.decompose_ref_unbounded() };
            if curr_tag == REMOVE_TAG {
                self.restart();
                continue;
            }

            let curr_next = curr.next();
            // (LIS:5) this `Acquire` load synchronizes-with the `Release` CAS (LIS:1) and (LIS:3)
            let next = curr_next.load(Acquire);

            if unsafe { self.prev.as_ref().load(Relaxed) } != MarkedPtr::from(curr) {
                self.restart();
                continue;
            }

            let (next, next_tag) = next.decompose();
            if next_tag == REMOVE_TAG && !self.ignore_marked(curr) {
                continue;
            }

            let prev = self.prev;
            self.prev = NonNull::from(curr_next);
            return Some(IterPos { prev, curr: NonNull::from(curr), next: NonNull::new(next) });
        }

        None
    }
}

/***** impl inherent ******************************************************************************/

impl<T> IterInner<'_, T> {
    #[inline]
    fn restart(&mut self) {
        self.prev = NonNull::from(self.head);
    }

    #[inline]
    fn ignore_marked(&self, curr: *const Node<T>) -> bool {
        match self.ignore {
            Some(ignore) if ignore.as_ptr() as *const _ == curr => true,
            _ => false,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IterPos
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug)]
struct IterPos<T> {
    prev: NonNull<AtomicMarkedPtr<Node<T>>>,
    curr: NonNull<Node<T>>,
    next: Option<NonNull<Node<T>>>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// UnwrapPtr (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

trait UnwrapPtr {
    type Item;

    fn unwrap_ptr(self) -> *mut Self::Item;
}

impl<T> UnwrapPtr for Option<NonNull<T>> {
    type Item = T;

    #[inline]
    fn unwrap_ptr(self) -> *mut Self::Item {
        match self {
            Some(non_null) => non_null.as_ptr(),
            None => ptr::null_mut(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering::Relaxed;
    use std::thread::{self, ThreadId};

    use super::List;

    static LIST: List<ThreadId> = List::new();

    #[test]
    fn thread_ids() {
        for _ in 0..100_000 {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    thread::spawn(|| {
                        let token = LIST.insert(thread::current().id());
                        let _ = LIST.remove(token); // leaks memory
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }

            assert!(LIST.head.load(Relaxed).is_null());
        }
    }
}
