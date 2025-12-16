.PHONY: build

build: target/arm-unknown-linux-musleabihf/release/tempmon

target/arm-unknown-linux-musleabihf/release/tempmon: ./src/main.rs
	cargo zigbuild --target=arm-unknown-linux-musleabihf --release
