WASM_TARGET := wasm32-wasip1
TOOLCHAIN := stable-x86_64-unknown-linux-gnu
RUSTC := $(HOME)/.rustup/toolchains/$(TOOLCHAIN)/bin/rustc
PLUGIN_DIR := $(HOME)/.config/zellij/plugins

.PHONY: build wasm install clean

build:
	cargo build --release

wasm:
	RUSTC=$(RUSTC) cargo build --release --target $(WASM_TARGET) -p rz-hub

install: build wasm
	cargo install --path crates/rz-cli
	mkdir -p $(PLUGIN_DIR)
	cp target/$(WASM_TARGET)/release/rz_hub.wasm $(PLUGIN_DIR)/

clean:
	cargo clean
