#![feature(test)]

extern crate test;

use std::mem;

use test::Bencher;

use crossbeam_utils::thread::scope;
use debra::{Config, Guard, CONFIG};

#[bench]
fn only_pin(b: &mut Bencher) {
    CONFIG.init_once(Config::with_params(128, 0));
    b.iter(|| {
        let guard = Guard::new();
        mem::forget(guard);
    })
}

#[bench]
fn single_pin(b: &mut Bencher) {
    CONFIG.init_once(Config::with_params(128, 0));
    b.iter(|| Guard::new());
}

#[bench]
fn multi_pin(b: &mut Bencher) {
    CONFIG.init_once(Config::with_params(128, 0));

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
