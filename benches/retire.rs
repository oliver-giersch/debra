#![feature(test)]

extern crate test;

use std::sync::atomic::Ordering::Relaxed;

use test::Bencher;

use debra::{ConfigBuilder, CONFIG};

type Atomic<T> = debra::Atomic<T, debra::typenum::U0>;
type Owned<T> = debra::Owned<T, debra::typenum::U0>;

#[bench]
fn retire(b: &mut Bencher) {
    CONFIG.init_once(|| ConfigBuilder::new().check_threshold(128).advance_threshold(0).build());

    let global = Atomic::new(1);

    b.iter(|| {
        let unlinked = global.swap(Owned::new(1), Relaxed).unwrap();
        unsafe { unlinked.retire() };
    });
}
