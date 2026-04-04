# pid1-rs

[![Crates.io][crates-badge]][crates-url]
[![Crates.io][crates-badge-exe]][crates-url-exe]
[![Rust](https://github.com/fpco/pid1-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/fpco/pid1-rs/actions/workflows/rust.yml)

[crates-badge]: https://img.shields.io/crates/v/pid1.svg
[crates-url]: https://crates.io/crates/pid1
[crates-badge-exe]: https://img.shields.io/crates/v/pid1-exe.svg
[crates-url-exe]: https://crates.io/crates/pid1-exe

A Rust library and binary for correct PID 1 signal handling and zombie
process reaping in containerized environments.

## Overview

In Unix-like systems, the process with Process ID (PID) 1 has special
responsibilities. It is the ancestor of all other processes and is
responsible for adopting orphaned processes and reaping them. When
running applications in containers, your application's process often
becomes PID 1.

Without proper handling, signals like `SIGTERM` might not be forwarded
to child processes, and zombie processes can accumulate, leading to
resource leaks. `pid1-rs` solves this by providing:

- **Signal Forwarding:** Intercepts signals like `SIGTERM` and
  `SIGINT` and forwards them to its child process, allowing for
  graceful shutdown.
- **Zombie Reaping:** Acts as an init process to reap orphaned child
  processes, preventing zombie process accumulation.

This project provides two ways to use this functionality: as a Rust
library integrated into your application, or as a standalone binary
executable.

## Comparison with `tini`

[tini](https://github.com/krallin/tini) is a popular, minimal init for containers. `pid1-rs`
provides similar functionality with a different approach:

- The **`pid1` library** integrates directly into your Rust
  application. This is its main advantage over `tini`, as you don't
  need to add a separate binary to your container. The PID 1 handling
  logic is compiled into your application, simplifying your
  `Dockerfile` and potentially reducing image size.
- The **`pid1-exe` binary** is a direct, Rust-native alternative to
  `tini`. Both serve the same purpose as a standalone init binary. If
  you prefer a toolchain built in Rust or need a init for non-Rust
  applications, `pid1-exe` is an excellent choice.

## Packages in this Repository

This repository consists of two packages:
- [`pid1`](./pid1/): A Rust library to integrate into your application.
- [`pid1-exe`](./pid1-exe): A standalone `pid1` binary for use in any container environment.

## `pid1` Library Usage

This library is used to simplify Rust deployment in a containerized
environment. Instead of using binaries like [Haskell's pid1](https://github.com/fpco/pid1) or
[tini](https://github.com/krallin/tini) in your container, you can use this crate directly.

### Usage

You must ensure that the `launch` method is the first statement in
your `main` function:

``` rust
use std::time::Duration;
use pid1::Pid1Settings;

fn main() {
    Pid1Settings::new()
        .enable_log(true) // Optional: for debugging
        .timeout(Duration::from_secs(2)) // Optional: timeout for graceful shutdown
        .launch()
        .expect("Launch failed");
    println!("Hello world");
    // Rest of the logic...
}
```

This function is meant only for Unix systems and the above code is a no-op
on Windows.

For more examples, see the [examples](./pid1/examples/) directory.

## `pid1-exe` Binary Usage

You can download the `pid1` binary from the [releases page](https://github.com/fpco/pid1-rs/releases) and use
it as the `ENTRYPOINT` in your container.

### Docker Example

In this example, `your-application` and its arguments are passed using
`CMD`.

``` dockerfile
FROM alpine:3.14.2

ADD --chmod=755 https://github.com/fpco/pid1-rs/releases/download/v0.1.0/pid1-x86_64-unknown-linux-musl /usr/bin/pid1

ENTRYPOINT [ "pid1" ]
CMD [ "your-application", "--arg1" ]
```

### Command-line Options

The `pid1` binary supports various command-line options:

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

---

## Development

The testing steps are documented in [Development.md](./Development.md).
