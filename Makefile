.PHONY: all
all: target/release/magic_pager.so target/release/mp

target/release/mp: $(wildcard src/*.rs)
	cargo build --release

target/release/magic_pager.so: src/preload.c
	gcc -O3 -shared -fPIC -Wall -o target/release/magic_pager.so src/preload.c -ldl
