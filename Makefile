QEMU_OPT?=
QEMU?=qemu-system-riscv64
BIOS?=none

default: build-riscv64

run: run-riscv64

gdb: gdb-riscv64

test: test-riscv64

expand-%:
	cargo expand --target riscv64gc-unknown-none-elf $*

clean:
	rm -rf target/

build-riscv64:
	cargo rustc \
		--release \
		--target riscv64gc-unknown-none-elf \
		-- \
		-C linker=riscv64-unknown-linux-gnu-gcc \
		-C link-arg=-nostartfiles \
		-C link-arg=-Tlink.ld \
		-C link-arg=boot.s \
		-C no-redzone=yes

run-riscv64: build-riscv64
	@echo Enter Ctrl-A + X to quit
	$(QEMU) \
		-s \
		-machine virt \
		-nographic \
		-m 32M \
		-smp 1 \
		-bios $(BIOS) \
		-kernel target/riscv64gc-unknown-none-elf/release/dux \
		$(QEMU_OPT)

gdb-riscv64:
	riscv64-unknown-linux-gnu-gdb \
		-ex='set arch riscv64' \
		-ex='target extended-remote localhost:1234' \
		target/riscv64gc-unknown-none-elf/release/dux

test-riscv64:
	cargo rustc \
		--release \
		--target riscv64gc-unknown-none-elf \
		-- \
		-C linker=riscv64-unknown-linux-gnu-gcc \
		-C link-arg=-nostartfiles \
		-C link-arg=-Tlink.ld \
		-C link-arg=boot.s \
		-C no-redzone=yes \
		--test
	@echo Enter Ctrl-A + X to quit
	$(QEMU) \
		-s \
		-machine virt \
		-nographic \
		-m 32M \
		-smp 1 \
		-bios $(BIOS) \
		-kernel target/riscv64gc-unknown-none-elf/release/dux \
		$(QEMU_OPT)


dump:
	riscv64-unknown-linux-gnu-objdump -SC target/riscv64gc-unknown-none-elf/release/dux
