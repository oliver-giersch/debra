//! A concurrent lock-free list that is ordered by the (heap) addresses of its
//! entries and does not deallocate memory of entries removed during its
//! lifetime.

//#[cfg(not(feature = "std"))]
//use alloc::boxed::Box;

use core::mem;
use core::ops::Deref;
use core::ptr::{self, NonNull};
use core::sync::atomic::Ordering::{self, Acquire, Relaxed, Release};

use reclaim::align::CacheAligned;
use reclaim::prelude::*;
use reclaim::typenum::U1;
use reclaim::{MarkedNonNull, MarkedPtr};

type AtomicMarkedPtr<T> = reclaim::AtomicMarkedPtr<T, U1>;

const REMOVE_TAG: usize = 0b1;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Queue
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A concurrent lock-free list with restricted permissions for entry removal.
///
/// Each entry in the queue is associated to an owner, represented by a
/// [`SetEntry`]. Only this owner can remove the entry again from the queue,
/// which may be located at an arbitrary position in the queue.
#[derive(Debug)]
pub(crate) struct List<T> {
    head: AtomicMarkedPtr<Node<T>>,
}

impl<T> List<T> {
    /// Creates a new empty list
    pub const fn new() -> Self {
        Self { head: AtomicMarkedPtr::null() }
    }

    /// Inserts the given `entry` and returns an owned [`SetEntry`] token, which
    /// can be used to remove the entry and also acts like a shared reference to
    /// it.
    ///
    /// Every entry is allocated as part of a [`Node`] on the heap and all
    /// entries are ordered by their respective heap addresses.
    #[inline]
    pub fn insert(&self, entry: T) -> ListEntry<T> {
        unsafe {
            let entry = Box::leak(Box::new(Node::new(entry)));
            loop {
                let mut iter = self.iter_inner();
                let InsertPos(prev, next) = iter
                    .find_map(|pos| pos.check_ordered_insert(entry))
                    .unwrap_or_else(|| InsertPos(iter.prev, None));

                let next = MarkedPtr::new(next.unwrap_ptr());
                entry.next().store(next, Relaxed);
                if prev
                    .as_ref()
                    .compare_exchange(next, MarkedPtr::new(entry), Release, Relaxed)
                    .is_ok()
                {
                    return ListEntry(NonNull::from(entry));
                }
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
        unsafe {
            loop {
                let pos = self
                    .iter_inner()
                    .find(|pos| pos.curr == entry)
                    .expect("given `entry` does not exist in this set");

                let next_unmarked = MarkedPtr::new(pos.next.unwrap_ptr());
                let next_marked = MarkedPtr::compose(pos.next.unwrap_ptr(), REMOVE_TAG);

                if pos
                    .curr
                    .as_ref()
                    .next
                    .compare_exchange(next_unmarked, next_marked, Acquire, Relaxed)
                    .is_err()
                {
                    continue;
                }

                if pos
                    .prev
                    .as_ref()
                    .compare_exchange(MarkedPtr::from(pos.curr), next_unmarked, Release, Relaxed)
                    .is_ok()
                {
                    return entry;
                }
            }
        }
    }

    /// Returns an iterator over the set.
    #[inline]
    pub fn iter(&self) -> Iter<T> {
        Iter(self.iter_inner())
    }

    #[inline]
    fn iter_inner(&self) -> IterInner<T> {
        IterInner { head: &self.head, prev: NonNull::from(&self.head) }
    }
}

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
pub(crate) struct ListEntry<T>(NonNull<Node<T>>);

impl<T> ListEntry<T> {
    #[inline]
    fn into_inner(self) -> NonNull<Node<T>> {
        let inner = self.0;
        mem::forget(self);
        inner
    }
}

impl<T> Deref for ListEntry<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let node = unsafe { &*self.0.as_ptr() };
        &*node.elem
    }
}

impl<T> Drop for ListEntry<T> {
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

impl<T> Node<T> {
    /// Returns a reference to the node's element.
    #[inline]
    pub fn elem(&self) -> &T {
        &*self.elem
    }

    #[inline]
    fn next(&self) -> &AtomicMarkedPtr<Node<T>> {
        &*self.next
    }

    #[inline]
    fn new(elem: T) -> Self {
        Self { elem: CacheAligned(elem), next: CacheAligned(AtomicMarkedPtr::null()) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Iter
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An iterator over a [`List`].
pub(crate) struct Iter<'a, T>(IterInner<'a, T>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|IterPos { curr, .. }| unsafe { &*curr.as_ptr() }.elem())
    }
}

impl<'a, T> Iter<'a, T> {
    /// Creates a new iterator for the given `list` that starts at the given
    /// list position.
    #[inline]
    pub fn new(list: &'a List<T>, start: &AtomicMarkedPtr<Node<T>>) -> Self {
        Self(IterInner { head: &list.head, prev: NonNull::from(start) })
    }

    /// Loads the entry and its tag at the current position of the iterator.
    ///
    /// # Errors
    ///
    /// Returns an error if a node is loaded whose predecessor is already marked
    /// for removal.
    #[inline]
    pub fn load_current(&mut self, order: Ordering) -> Result<Option<&T>, IterError> {
        let (curr, tag) = unsafe { self.0.prev.as_ref().load(order).decompose_ref() };
        if tag == REMOVE_TAG {
            Err(IterError::Retry)
        } else {
            Ok(curr)
        }
    }

    #[inline]
    pub fn load_head(&self, order: Ordering) -> Option<&T> {
        unsafe { self.0.head.load(order).as_ref() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IterError
////////////////////////////////////////////////////////////////////////////////////////////////////

pub(crate) enum IterError {
    Retry,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IterInner
////////////////////////////////////////////////////////////////////////////////////////////////////

struct IterInner<'a, T> {
    head: &'a AtomicMarkedPtr<Node<T>>,
    prev: NonNull<AtomicMarkedPtr<Node<T>>>,
}

impl<T> Iterator for IterInner<'_, T> {
    type Item = IterPos<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            while let Value(curr) = MarkedNonNull::new(self.prev.as_ref().load(Acquire)) {
                let (curr, curr_tag) = curr.decompose_ref_unbounded();
                if curr_tag == 0b1 {
                    self.prev = NonNull::from(self.head);
                    continue;
                }

                let curr_next = curr.next();
                let next = curr_next.load(Acquire);

                if self.prev.as_ref().load(Relaxed) != MarkedPtr::from(curr) {
                    self.prev = NonNull::from(self.head);
                    continue;
                }

                let (next, next_tag) = next.decompose();
                if next_tag == REMOVE_TAG {
                    continue;
                }

                self.prev = NonNull::from(curr_next);
                return Some(IterPos {
                    prev: self.prev,
                    curr: NonNull::from(curr),
                    next: NonNull::new(next),
                });
            }

            None
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

impl<T> IterPos<T> {
    #[inline]
    fn check_ordered_insert(&self, other: &Node<T>) -> Option<InsertPos<T>> {
        if self.curr > NonNull::from(other) {
            Some(InsertPos(self.prev, Some(self.curr)))
        } else {
            None
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// InsertPos
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
struct InsertPos<T>(NonNull<AtomicMarkedPtr<Node<T>>>, Option<NonNull<Node<T>>>);

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
