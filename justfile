# List all recipies
default:
	just --list --unsorted

# Test
test:
	cargo build --target x86_64-unknown-linux-musl --example simple
	cp target/x86_64-unknown-linux-musl/debug/examples/simple etc
	docker build etc -f etc/Dockerfile --tag pid1rstest
	docker rm pid1rs || exit 0
	docker run --name pid1rs -t pid1rstest

# Exec into that docker container
exec-shell:
	docker exec -it pid1rs sh
