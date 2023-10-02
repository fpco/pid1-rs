# List all recipies
default:
	just --list --unsorted

# Build pid binary
build-release-binary:
	cargo build --target x86_64-unknown-linux-musl --release

# Build test container
test: build-release-binary
	cp target/x86_64-unknown-linux-musl/release/pid1 ./pid1-exe/etc/
	cd pid1-exe/etc && docker build . -f Dockerfile --tag pid1runner

# Test docker image
test-init-image:
	docker run --rm --interactive --name pid pid1runner ps aux
	docker run --rm --interactive --name pid pid1runner ls
	docker run --rm --interactive --name pid pid1runner ls /
	docker run --rm --interactive --name pid pid1runner id
	docker run --rm --interactive --name pid pid1runner --workdir=/home  pwd
	docker run --rm --interactive --name pid pid1runner --env HELLO=WORLD --env=FOO=BYE printenv HELLO FOO

# Exec init image
exec-init-image:
	docker run --rm --name pid --tty --interactive pid1runner sh

# Build binary for other architectures
binaries clean='false':
	cross build --target x86_64-unknown-linux-gnu --release
	cross build --target aarch64-unknown-linux-gnu --release
	cross build --target aarch64-unknown-linux-musl --release
	cross build --target arm-unknown-linux-musleabi --release
	cross build --target arm-unknown-linux-musleabihf --release
	cross build --target armv5te-unknown-linux-musleabi --release
	cross build --target armv7-unknown-linux-musleabi --release
	cross build --target armv7-unknown-linux-musleabihf --release
	cross build --target i586-unknown-linux-musl --release
	cross build --target i686-unknown-linux-musl --release
	{{clean}} && docker image rm ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5
	{{clean}} && docker image rm ghcr.io/cross-rs/aarch64-unknown-linux-gnu:0.2.5
	{{clean}} && docker image rm ghcr.io/cross-rs/aarch64-unknown-linux-musl:0.2.5
	{{clean}} && docker image rm ghcr.io/cross-rs/arm-unknown-linux-musleabi:0.2.5
	{{clean}} && docker image rm ghcr.io/cross-rs/arm-unknown-linux-musleabihf:0.2.5
	{{clean}} && docker image rm ghcr.io/cross-rs/armv5te-unknown-linux-musleabi:0.2.5
	{{clean}} && docker image rm ghcr.io/cross-rs/armv7-unknown-linux-musleabi:0.2.5
	{{clean}} && docker image rm ghcr.io/cross-rs/armv7-unknown-linux-musleabihf:0.2.5
	{{clean}} && docker image rm ghcr.io/cross-rs/i586-unknown-linux-musl:0.2.5
	{{clean}} && docker image rm ghcr.io/cross-rs/i686-unknown-linux-musl:0.2.5
	cross build --target mips64-unknown-linux-muslabi64 --release
	cross build --target mips64el-unknown-linux-muslabi64 --release

# Copy binaries to artifacts directory
cp-binaries:
	mkdir -p artifacts
	cp target/x86_64-unknown-linux-musl/release/pid1  ./artifacts/pid1-x86_64-unknown-linux-musl
	cp target/x86_64-unknown-linux-gnu/release/pid1 ./artifacts/pid1-x86_64-unknown-linux-gnu
	cp target/aarch64-unknown-linux-gnu/release/pid1 ./artifacts/pid1-aarch64-unknown-linux-gnu
	cp target/aarch64-unknown-linux-musl/release/pid1 ./artifacts/pid1-aarch64-unknown-linux-musl
	cp target/arm-unknown-linux-musleabi/release/pid1 ./artifacts/pid1-arm-unknown-linux-musleabi
	cp target/arm-unknown-linux-musleabihf/release/pid1 ./artifacts/pid1-arm-unknown-linux-musleabihf
	cp target/armv5te-unknown-linux-musleabi/release/pid1 ./artifacts/pid1-armv5te-unknown-linux-musleabi
	cp target/armv7-unknown-linux-musleabi/release/pid1 ./artifacts/pid1-armv7-unknown-linux-musleabi
	cp target/armv7-unknown-linux-musleabihf/release/pid1 ./artifacts/pid1-armv7-unknown-linux-musleabihf
	cp target/i586-unknown-linux-musl/release/pid1 ./artifacts/pid1-i586-unknown-linux-musl
	cp target/i686-unknown-linux-musl/release/pid1 ./artifacts/pid1-i686-unknown-linux-musl
	cp target/mips64-unknown-linux-muslabi64/release/pid1 ./artifacts/pid1-mips64-unknown-linux-muslabi64
	cp target/mips64el-unknown-linux-muslabi64/release/pid1 ./artifacts/pid1-mips64el-unknown-linux-muslabi64
	file artifacts/*
