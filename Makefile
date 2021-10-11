all: release_musl

release:
	cargo build --release --bins

release_musl:
	cargo build --release --bins --target=x86_64-unknown-linux-musl

build:
	cargo build --release --bins

build_musl:
	cargo build --release --bins --target=x86_64-unknown-linux-musl

lint:
	cargo clippy

test:
	cargo test -- --test-threads=1

fmt:
	cargo fmt

clean:
	cargo clean

cleanall: clean
	git clean -fdx
