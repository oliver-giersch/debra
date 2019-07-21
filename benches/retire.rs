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

#[bench]
fn retire_varied(b: &mut Bencher) {
    CONFIG.init_once(|| ConfigBuilder::new().check_threshold(128).advance_threshold(0).build());

    let int = Atomic::new(1);
    let string = Atomic::new(String::from("string"));
    let arr = Atomic::new([0usize; 16]);

    b.iter(|| unsafe {
        int.swap(Owned::new(1), Relaxed).unwrap().retire();
        string.swap(Owned::new(String::from("string")), Relaxed).unwrap().retire();
        arr.swap(Owned::new([0usize; 16]), Relaxed).unwrap().retire();
    });
}
