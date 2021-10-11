all: pack

export CARGO_NET_GIT_FETCH_WITH_CLI = true

BUILD_DIR = build_dir
PACK_DIR = /tmp/btm.package
PACK_TARGET= btm.tar.gz

release:
	@ cargo build --release --bins --target-dir=$(BUILD_DIR)

release_musl:
	@ cargo build --release --bins --target=x86_64-unknown-linux-musl --target-dir=$(BUILD_DIR)

build:
	@ cargo build --release --bins --target-dir=$(BUILD_DIR)

build_musl:
	@ cargo build --release --bins --target=x86_64-unknown-linux-musl --target-dir=$(BUILD_DIR)

lint:
	cargo clippy

test:
	cargo test -- --test-threads=1

fmt:
	sh tools/fmt.sh

pack: release_musl
	@ rm -rf $(PACK_DIR)
	@ mkdir -p $(PACK_DIR)
	@ cp tools/install.sh $(PACK_DIR)/
	@ cp tools/btm-daemon.service $(PACK_DIR)/
	@ cp $(BUILD_DIR)/x86_64-unknown-linux-musl/release/btm $(PACK_DIR)/
	@ tar -zcpf $(PACK_TARGET) $(PACK_DIR)
	@ echo -e "\n\033[31;01mbuild path:\033[0m $(BUILD_DIR)"
	@ echo -e "\033[31;01mpackage path:\033[0m $(PACK_TARGET)\n"

update:
	cargo update

clean:
	cargo clean
	rm -rf $(BUILD_DIR) $(PACK_TARGET)

cleanall: clean
	git clean -fdx
