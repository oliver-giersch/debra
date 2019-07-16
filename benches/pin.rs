#![feature(test)]

extern crate test;

use std::mem;

use test::Bencher;

use crossbeam_utils::thread::scope;
use debra::{ConfigBuilder, Guard, CONFIG};

#[ignore]
#[bench]
fn only_pin(b: &mut Bencher) {
    CONFIG.init_once(|| ConfigBuilder::new().check_threshold(128).advance_threshold(0).build());
    b.iter(|| {
        let guard = Guard::new();
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
