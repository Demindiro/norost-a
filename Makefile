KERNEL?=target/riscv64gc-unknown-none-elf/release/dux
KERNEL_DEBUG?=target/riscv64gc-unknown-none-elf/debug/dux

QEMU_OPT?=
QEMU_BIOS?=none
QEMU?=qemu-system-riscv64 \
		-s \
		-machine virt \
		-nographic \
		-m 32M \
		-smp 1 \
		-bios $(QEMU_BIOS) \
		-kernel $(KERNEL)
QEMU_DEBUG?=qemu-system-riscv64 \
		-s \
		-machine virt \
		-nographic \
		-m 32M \
		-smp 1 \
		-bios $(QEMU_BIOS) \
		-kernel $(KERNEL_DEBUG)

CARGO?=cargo rustc \
	--release \
	--target riscv64gc-unknown-none-elf \
	-- \
	-C linker=riscv64-unknown-linux-gnu-gcc \
	-C link-arg=-nostartfiles \
	-C link-arg=-Tlink.ld \
	-C link-arg=boot.s \
	-C no-redzone=yes

CARGO_DEBUG?=cargo rustc \
	--target riscv64gc-unknown-none-elf \
	-- \
	-C linker=riscv64-unknown-linux-gnu-gcc \
	-C link-arg=-nostartfiles \
	-C link-arg=-Tlink.ld \
	-C link-arg=boot.s \
	-C no-redzone=yes

default: build-riscv64

run: run-riscv64

run-debug: run-debug-riscv64

gdb: gdb-riscv64

gdb-debug: gdb-debug-riscv64

test: test-riscv64

test-debug: test-debug-riscv64

dump: dump-riscv64

dump-debug: dump-debug-riscv64

expand-%:
	cargo expand --target riscv64gc-unknown-none-elf $*

clean:
	rm -rf target/

build-riscv64:
	$(CARGO)

build-debug-riscv64:
	$(CARGO_DEBUG)

run-riscv64: build-riscv64
	@echo Enter Ctrl-A + X to quit
	$(QEMU) $(QEMU_OPT)

run-debug-riscv64: build-debug-riscv64
	@echo Enter Ctrl-A + X to quit
	$(QEMU_DEBUG) $(QEMU_OPT)

gdb-riscv64:
	riscv64-unknown-linux-gnu-gdb \
		-ex='set arch riscv64' \
		-ex='target extended-remote localhost:1234' \
		$(KERNEL)

gdb-debug-riscv64:
	riscv64-unknown-linux-gnu-gdb \
		-ex='set arch riscv64' \
		-ex='target extended-remote localhost:1234' \
		$(KERNEL_DEBUG)

test-riscv64:
	$(CARGO) --test
	@echo Enter Ctrl-A + X to quit
	$(QEMU) $(QEMU_OPT)

test-debug-riscv64:
	$(CARGO_DEBUG) --test
	@echo Enter Ctrl-A + X to quit
	$(QEMU_DEBUG) $(QEMU_OPT)

dump-riscv64:
	riscv64-unknown-linux-gnu-objdump -SC $(KERNEL)

dump-debug-riscv64:
	riscv64-unknown-linux-gnu-objdump -SC $(KERNEL_DEBUG)
