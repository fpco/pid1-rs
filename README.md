# pid1-rs

[![Rust](https://github.com/fpco/pid1-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/fpco/pid1-rs/actions/workflows/rust.yml)

pid1 handling library for proper signal and zombie reaping of the PID1
process.

This repository consists of two packages:
- [pid1](./pid1/) crate: Library meant to be used by your Rust applications.
- [pid1-exe](./pid1-exe) crate: Binary which internally uses pid1
  crate for container deployments. The binary name is `pid1`.

## pid1 Library Usage

This library is used to simplify Rust deployment in a containerized
environment. Instead of using binaries like [Haskell's pid1](https://github.com/fpco/pid1-rs/actions/workflows/rust.yml)) or
[tini](https://github.com/krallin/tini) in your container, you can use this crate directly.

You must ensure that the `launch` method is the first statement in
your `main` function:

``` rust
use std::time::Duration;
use pid1::Pid1Settings;

fn main() {
    Pid1Settings::new()
        .enable_log(true)
        .timeout(Duration::from_secs(2))
        .launch()
        .expect("Launch failed");
    println!("Hello world");
    // Rest of the logic...
}
```

You can also see various example usages [here.](./examples/) This
function is meant only for Unix systems and the above code is a no-op
in Windows.

## Using pid1 binary

You can download the `pid1` binary that is part of the [releases](https://github.com/fpco/pid1-rs/releases)
and use it in your container directly. Example:

``` dockerfile
FROM alpine:3.14.2

ADD FIXME_LINK /usr/bin/pid1

RUN chmod +x /usr/bin/pid1

ENTRYPOINT [ "pid1" ]
```

## Development

The testing steps are documented in [Development.md](./Development.md). We only have
some part of it integrated in CI.
