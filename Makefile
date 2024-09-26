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
	cargo fmt
	cargo clippy

test:
	cargo test -- --test-threads=1

fmt:
	cargo fmt

fmtall:
	bash tools/fmt.sh 2>/dev/null

pack:
	if [ "Linux" = `uname -s` ]; then \
		if [ 0 -eq `rustup target list --installed | grep -c 'musl'` ]; \
			then rustup target add x86_64-unknown-linux-musl; \
		fi; \
		$(MAKE) release_musl; \
	else \
		$(MAKE) release; \
	fi
	@ rm -rf $(PACKAGE)
	@ mkdir -p $(PACKAGE)
	@ cp tools/install.sh $(PACKAGE)/
	@ cp tools/btm-daemon.service $(PACKAGE)/
	if [ "Linux" = `uname -s` ]; then \
		cp $(BUILD_DIR)/x86_64-unknown-linux-musl/release/btm $(PACKAGE)/; \
	else \
		cp $(BUILD_DIR)/release/btm $(PACKAGE)/; \
	fi
	cp $(PACKAGE)/btm ~/.cargo/bin/
	@ tar -zcpf $(PACKAGE_TARGET) $(PACKAGE)
	@ printf "\n\033[31;01mbuild path:\033[0m $(BUILD_DIR)\n"
	@ printf "\033[31;01mpackage path:\033[0m $(PACKAGE_TARGET)\n"

update:
	cargo update --verbose

clean:
	cargo clean
	rm -rf $(BUILD_DIR) $(PACKAGE) $(PACKAGE_TARGET)

cleanall: clean
	git clean -fdx
