all: pack

BUILD_DIR = target
PACKAGE = btm_package
PACKAGE_TARGET= $(PACKAGE).tar.gz

export CARGO_NET_GIT_FETCH_WITH_CLI = true

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
	bash tools/fmt.sh 2>/dev/null

pack: release_musl
	@ rm -rf $(PACKAGE)
	@ mkdir -p $(PACKAGE)
	@ cp tools/install.sh $(PACKAGE)/
	@ cp tools/btm-daemon.service $(PACKAGE)/
	@ cp $(BUILD_DIR)/x86_64-unknown-linux-musl/release/btm $(PACKAGE)/
	@ tar -zcpf $(PACKAGE_TARGET) $(PACKAGE)
	@ printf "\n\033[31;01mbuild path:\033[0m $(BUILD_DIR)\n"
	@ printf "\033[31;01mpackage path:\033[0m $(PACKAGE_TARGET)\n"

update:
	cargo update

clean:
	cargo clean
	rm -rf $(BUILD_DIR) $(PACKAGE) $(PACKAGE_TARGET)

cleanall: clean
	git clean -fdx
