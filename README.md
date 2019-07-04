# Debra

Distributed epoch-based memory reclamation

[![Build Status](https://travis-ci.com/oliver-giersch/debra.svg?branch=master)](
https://travis-ci.com/oliver-giersch/debra)
[![Latest version](https://img.shields.io/crates/v/debra.svg)](https://crates.io/crates/debra)
[![Documentation](https://docs.rs/debra/badge.svg)](https://docs.rs/debra)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/oliver-giersch/debra)
[![Rust 1.36+](https://img.shields.io/badge/rust-1.36+-lightgray.svg)](
https://www.rust-lang.org)

Many concurrent lock-free data structures require an additional minimal (also lock-free)
garbage collector, which determines, when a removed value can be safely de-allocated.
This can not be determined statically, since many threads could potentially still access
previously created references to the removed value.
This crate provides a simple and (mostly) safe interface for interacting with the
[DEBRA](https://dl.acm.org/citation.cfm?id=2767436) memory reclamation scheme.

## Usage

Add this to your `Cargo.toml`

```
[dependencies]
debra = "0.1"
```

## Minimum Supported Rust Version (MSRV)

The minimum supported (stable) rust version for this crate is 1.36.0

## Comparison with [crossbeam-epoch](https://crates.io/crates/crossbeam-epoch)

...TODO...

## Examples

See [tests/treiber.rs](tests/treiber.rs) for an implementation
of Treiber's stack using `debra` for memory reclamation.

## Features

...TODO...
DEBRA_CHECK_THRESHOLD
DEBRA_ADVANCE_THRESHOLD

## License

Debra is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
