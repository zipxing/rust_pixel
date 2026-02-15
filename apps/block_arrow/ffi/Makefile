GCC_BIN ?= $(shell which g++)
CARGO_BIN ?= $(shell which cargo)

gen:
	cbindgen --config cbindgen.toml -l c++ --crate block_arrow_ffi --output csrc/block_arrow_ffi.h
	cbindgen --config cbindgen_py.toml -l c --crate block_arrow_ffi --output python/pffi.h

leak: 
	leaks -atExit -- csrc/test

run: gen clean build 
	./csrc/test
	cd python;python3 testffi.py

clean: 
	$(CARGO_BIN) clean -p block_arrow_ffi
	rm -f ./csrc/test

build:
	$(CARGO_BIN) build --release
	$(GCC_BIN) -std=c++11 -Wall -o ./csrc/test ./csrc/test.cc -I . -L ./target/release -lblock_arrow_ffi
build_unity:
	$(CARGO_BIN) build --target aarch64-linux-android --release
	$(CARGO_BIN) build --target armv7-linux-androideabi --release
	# $(CARGO_BIN) build --target i686-linux-android --release
build_ios:
	cargo build --target aarch64-apple-ios  --release
	cargo build --target aarch64-apple-ios-sim  --release
copy_to_dest:
	cp ../../target/aarch64-linux-android/release/libblock_arrow_ffi.so /Users/tuyoo_zzg/learn/rust/unity/RustFFI/Assets/Plugins/RustFFI/Arm64
	cp ../../target/armv7-linux-androideabi/release/libblock_arrow_ffi.so /Users/tuyoo_zzg/learn/rust/unity/RustFFI/Assets/Plugins/RustFFI/Armv7a
	cp ../../target/release/libblock_arrow_ffi.dylib /Users/tuyoo_zzg/learn/rust/unity/RustFFI/Assets/Plugins/RustFFI

