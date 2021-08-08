include ../../../common.mk

NAME = b0

build:
	cargo build --release

readelf:
	$(READELF) -a ../../../target/riscv64gc-unknown-none-elf/release/$(NAME)

objdump:
	$(OBJDUMP) -SC ../../../target/riscv64gc-unknown-none-elf/release/$(NAME)

symlink:
	rm -f ../../../init.elf
	ln -s target/riscv64gc-unknown-none-elf/release/b0 ../../../init.elf
