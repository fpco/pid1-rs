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
	docker run --rm --name pid pid1runner ps aux
	docker run --rm --name pid pid1runner ls
	docker run --rm --name pid pid1runner ls /
	docker run --rm --name pid pid1runner id
	docker run --rm --name pid pid1runner --workdir=/home  pwd
	docker run --rm --name pid pid1runner --env HELLO=WORLD --env=FOO=BYE printenv HELLO FOO

# Exec init image
exec-init-image:
	docker run --rm --name pid --tty --interactive pid1runner sh
