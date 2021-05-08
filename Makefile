QEMU_OPT?=

default: build-riscv64

run: run-riscv64

gdb: gdb-riscv64

test:
	cargo test

expand-%:
	cargo expand --target riscv64gc-unknown-none-elf $*

clean:
	rm -r target/

build-riscv64:
	cargo rustc \
		--release \
		--target riscv64gc-unknown-none-elf \
		-- \
		-C linker=riscv64-linux-gnu-gcc \
		-C link-arg=-nostartfiles \
		-C link-arg=-Tlink.ld \

run-riscv64: build-riscv64
	@echo Enter Ctrl-A + X to quit
	qemu-system-riscv64 \
		-s \
		-machine virt \
		-nographic \
		-m 32M \
		-smp 1 \
		-bios none \
		-kernel target/riscv64gc-unknown-none-elf/release/dux \
		$(QEMU_OPT)

gdb-riscv64:
	gdb \
		-ex='set arch riscv64' \
		-ex='target extended-remote localhost:1234' \
		target/riscv64gc-unknown-none-elf/release/dux

dump:
	riscv64-linux-gnu-objdump -SC target/riscv64gc-unknown-none-elf/release/dux
