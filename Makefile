.PHONY: build

SOURCES := $(wildcard src/*.rs)

build: target/arm-unknown-linux-musleabihf/release/tempmon

target/arm-unknown-linux-musleabihf/release/tempmon: $(SOURCES) Cargo.toml
	cargo zigbuild --target=arm-unknown-linux-musleabihf --release
