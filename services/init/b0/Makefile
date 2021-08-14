include ../../../common.mk
include ../../../common_rust.mk

NAME = b0

build:
	cargo build --release

symlink:
	rm -f ../../../init.elf
	ln -s target/riscv64gc-unknown-none-elf/release/b0 ../../../init.elf
