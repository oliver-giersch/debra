use std::mem::{self, ManuallyDrop};
use std::ptr;
use std::sync::atomic::{
    AtomicUsize,
    Ordering::{Acquire, Relaxed, Release},
};
use std::sync::Arc;
use std::thread;

use debra::{Guard, Owned};

type Atomic<T> = debra::Atomic<T, debra::typenum::U0>;

struct Stack<T> {
    head: Atomic<Node<T>>,
}

impl<T> Stack<T> {
    #[inline]
    pub fn new() -> Self {
        Self { head: Atomic::null() }
    }

    #[inline]
    pub fn push(&self, elem: T) {
        let mut node = Owned::new(Node::new(elem));
        let guard = &Guard::new();

        loop {
            let head = self.head.load(Acquire, guard);
            node.next.store(head, Relaxed);

            match self.head.compare_exchange_weak(head, node, Release, Relaxed) {
                Ok(_) => return,
                Err(fail) => node = fail.input,
            };
        }
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        let guard = &Guard::new();

        while let Some(head) = self.head.load(Relaxed, guard) {
            let next = head.next.load_unprotected(Relaxed);
            if let Ok(unlinked) = self.head.compare_exchange_weak(head, next, Release, Relaxed) {
                unsafe {
                    // the `Drop` code for T is never called for retired nodes, so it is
                    // safe to use `retire_unchecked` and not require that `T: 'static`.
                    let elem = ptr::read(&*unlinked.elem);
                    unlinked.retire_unchecked();
                    return Some(elem);
                }
            }
        }

        None
    }
}

impl<T> Drop for Stack<T> {
    #[inline]
    fn drop(&mut self) {
        let mut curr = self.head.take();
        while let Some(mut node) = curr {
            unsafe { ManuallyDrop::drop(&mut node.elem) };
            curr = node.next.take();
        }
    }
}

#[derive(Debug)]
struct Node<T> {
    elem: ManuallyDrop<T>,
    next: Atomic<Node<T>>,
}

impl<T> Node<T> {
    #[inline]
    fn new(elem: T) -> Self {
        Self { elem: ManuallyDrop::new(elem), next: Atomic::null() }
    }
}

#[repr(align(64))]
struct ThreadCount(AtomicUsize);

struct DropCount<'a>(&'a AtomicUsize);
impl Drop for DropCount<'_> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Relaxed);
    }
}

#[test]
fn treiber_stack() {
    const THREADS: usize = 8;
    const INITIAL: usize = 1_000;
    const OPERATIONS: usize = 1_000_000;
    const PER_THREAD_ALLOCATIONS: usize = OPERATIONS + INITIAL;
    static COUNTERS: [ThreadCount; THREADS] = [
        ThreadCount(AtomicUsize::new(0)),
        ThreadCount(AtomicUsize::new(0)),
        ThreadCount(AtomicUsize::new(0)),
        ThreadCount(AtomicUsize::new(0)),
        ThreadCount(AtomicUsize::new(0)),
        ThreadCount(AtomicUsize::new(0)),
        ThreadCount(AtomicUsize::new(0)),
        ThreadCount(AtomicUsize::new(0)),
    ];

    let stack = Arc::new(Stack::new());
    let handles: Vec<_> = (0..THREADS)
        .map(|id| {
            let stack = Arc::clone(&stack);
            thread::spawn(move || {
                let counter = &COUNTERS[id].0;

                for _ in 0..INITIAL {
                    stack.push(DropCount(counter));
                }

                for _ in 0..OPERATIONS {
                    let _res = stack.pop();
                    stack.push(DropCount(counter));
                }

                println!(
                    "thread {} reclaimed {:7} records before exiting",
                    id,
                    counter.load(Relaxed)
                );
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    mem::drop(stack);
    let drop_sum = COUNTERS.iter().map(|local| local.0.load(Relaxed)).sum();

    assert_eq!(THREADS * PER_THREAD_ALLOCATIONS, drop_sum);
    println!("total dropped records: {}, no memory was leaked", drop_sum);
}
