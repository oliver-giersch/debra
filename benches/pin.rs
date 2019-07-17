#![feature(test)]

extern crate test;

use std::mem;
use std::sync::atomic::Ordering::Relaxed;

use test::Bencher;

use crossbeam_utils::thread::scope;
use debra::{ConfigBuilder, Guard, CONFIG};

type Atomic<T> = debra::Atomic<T, debra::typenum::U0>;

#[ignore]
#[bench]
fn only_pin(b: &mut Bencher) {
    CONFIG.init_once(|| ConfigBuilder::new().check_threshold(128).advance_threshold(0).build());
    b.iter(|| {
        let guard = Guard::new();
        // this appears to mess with the other benchmarks, so only_pin should be run in isolation
        mem::forget(guard);
    })
}

#[bench]
fn single_pin(b: &mut Bencher) {
    CONFIG.init_once(|| ConfigBuilder::new().check_threshold(128).advance_threshold(0).build());
    b.iter(Guard::new);
}

#[bench]
fn multi_pin(b: &mut Bencher) {
    CONFIG.init_once(|| ConfigBuilder::new().check_threshold(128).advance_threshold(0).build());

    const THREADS: usize = 16;
    const STEPS: usize = 100_000;

    b.iter(|| {
        scope(|s| {
            for _ in 0..THREADS {
                s.spawn(|_| {
                    for _ in 0..STEPS {
                        Guard::new();
                    }
                });
            }
        })
        .unwrap();
    });
}

#[bench]
fn pin_and_load(b: &mut Bencher) {
    let atomic = Atomic::new(1);

    b.iter(|| {
        let guard = &Guard::new();
        assert_eq!(*atomic.load(Relaxed, guard).unwrap(), 1);
    })
}
