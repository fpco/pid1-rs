# pid1-rs

[![Crates.io][crates-badge]][crates-url]
[![Crates.io][crates-badge-exe]][crates-url-exe]
[![Rust](https://github.com/fpco/pid1-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/fpco/pid1-rs/actions/workflows/rust.yml)

[crates-badge]: https://img.shields.io/crates/v/pid1.svg
[crates-url]: https://crates.io/crates/pid1
[crates-badge-exe]: https://img.shields.io/crates/v/pid1-exe.svg
[crates-url-exe]: https://crates.io/crates/pid1-exe

pid1 handling library for proper signal and zombie reaping of the PID1
process.

This repository consists of two packages:
- [pid1](./pid1/) crate: Library meant to be used by your Rust applications.
- [pid1-exe](./pid1-exe) crate: Binary which internally uses pid1
  crate for container deployments. The binary name is `pid1`.

## pid1 Library Usage

This library is used to simplify Rust deployment in a containerized
environment. Instead of using binaries like [Haskell's
pid1](https://github.com/fpco/pid1) or
[tini](https://github.com/krallin/tini) in your container, you can use
this crate directly.

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

ADD https://github.com/fpco/pid1-rs/releases/download/v0.1.0/pid1-x86_64-unknown-linux-musl /usr/bin/pid1

RUN chmod +x /usr/bin/pid1

ENTRYPOINT [ "pid1" ]
```

Various options supported by the binary:

``` shellsession
❯ pid1 --help
Usage:

Arguments:
  <COMMAND>  Process to run
  [ARGS]...  Arguments to the process

Options:
  -w, --workdir <DIR>        Specify working direcory
  -t, --timeout <TIMEOUT>    Timeout (in seconds) to wait for child proess to exit [default: 2]
  -v, --verbose              Turn on verbose output
  -e, --env <ENV>            Override environment variables. Can specify multiple times
  -u, --user-id <USER_ID>    Run command with user ID
  -g, --group-id <GROUP_ID>  Run command with group ID
  -h, --help                 Print help
```

## Development

The testing steps are documented in [Development.md](./Development.md). We only have
some part of it integrated in CI.
