# List all recipies
default:
	just --list --unsorted

# Basic test
test:
    cargo build --target x86_64-unknown-linux-musl --example simple
    cp target/x86_64-unknown-linux-musl/debug/examples/simple etc
    docker build etc --tag pid1rstest
    docker run --rm -t pid1rstest
