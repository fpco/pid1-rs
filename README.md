# pid1-rs

[![Rust](https://github.com/fpco/pid1-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/fpco/pid1-rs/actions/workflows/rust.yml)

pid1 handling library for proper signal and zombie reaping of the PID1
process.

This library is used to simplify Rust deployment in a containerized
environment. Instead of using something like pid1 or tini binary in
your container, you can directly use this crate.

## Usage

You need to ensure that the function `relaunch_if_pid1` should be the
initial statement within your `main` function:

``` rust
use std::time::Duration;
use pid1::Builder;

fn main()
{
    let mut builder = Builder::new();
    let builder = builder
        .timeout(Duration::from_secs(2))
        .enable_log(true)
        .build();
    pid1::relaunch_if_pid1(builder)?;
    println!("Hello world");
    // Rest of the logic...
}
```

You can also see various example usages [here.](./examples/) This function is
meant only for Unix systems and is a no-op in Windows.

## Development

The testing steps are documented in [Development.md](./Development.md). We only have
some part of it integrated in CI.
