# List all recipies
default:
	just --list --unsorted

# Build pid binary
build-release-binary:
	cargo build --target x86_64-unknown-linux-musl --release

# Build test container
test: build-release-binary
	cp target/x86_64-unknown-linux-musl/release/pid1 ./init/etc/
	cd init/etc && docker build . -f Dockerfile --tag pid1runner
	-docker rm pid
	docker run --rm --name pid --tty pid1runner ps aux
