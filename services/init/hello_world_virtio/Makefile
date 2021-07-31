export CARGO_BUILD_TARGET = riscv64gc-unknown-none-elf

build:
	cargo build --release $(CARGO_OPT)

readelf:
	riscv64-unknown-linux-gnu-readelf -a ../../../target/riscv64gc-unknown-none-elf/release/hello_world_virtio

objdump:
	riscv64-unknown-linux-gnu-objdump -SC ../../../target/riscv64gc-unknown-none-elf/release/hello_world_virtio

symlink:
	rm -f ../../../init.elf
	ln -s target/riscv64gc-unknown-none-elf/release/hello_world_virtio ../../../init.elf
