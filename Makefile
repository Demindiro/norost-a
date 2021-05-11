DRIVE?=drive.bin

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
		-drive file=$(DRIVE),format=raw \
		-kernel $(KERNEL)
QEMU_DEBUG?=qemu-system-riscv64 \
		-s \
		-machine virt \
		-nographic \
		-m 32M \
		-smp 1 \
		-bios $(QEMU_BIOS) \
		-drive file=$(DRIVE),format=raw \
		-kernel $(KERNEL_DEBUG)

CARGO_OPT?=
CARGO?=cargo rustc \
	--release \
	--target riscv64gc-unknown-none-elf \
	$(CARGO_OPT) \
	-- \
	-C linker=riscv64-unknown-linux-gnu-gcc \
	-C link-arg=-nostartfiles \
	-C link-arg=-Tlink.ld \
	-C link-arg=boot.s \
	-C no-redzone=yes
CARGO_DEBUG?=cargo rustc \
	--target riscv64gc-unknown-none-elf \
	$(CARGO_OPT) \
	-- \
	-C linker=riscv64-unknown-linux-gnu-gcc \
	-C link-arg=-nostartfiles \
	-C link-arg=-Tlink.ld \
	-C link-arg=boot.s \
	-C no-redzone=yes

OBJDUMP_OPT?=-S

NM_OPT?=

READELF_OPT?=-a

default: build
	
build: build-riscv64

build-debug: build-debug-riscv64

run: run-riscv64

run-debug: run-debug-riscv64

gdb: gdb-riscv64

gdb-debug: gdb-debug-riscv64

test: test-riscv64

test-debug: test-debug-riscv64

objdump: objdump-riscv64

objdump-debug: objdump-debug-riscv64

nm: nm-riscv64

readelf: readelf-riscv64

strip: strip-riscv64

measure-stack-size: build
	@echo Enter Ctrl-A + X to exit
	$(QEMU) -d cpu,nochain 2>&1 \
		| rg sp \
		| sed 's/.*sp   \(.*\) gp.*/\1/g' \
		| sed 's/^0*//g' \
		| rg . \
		| uniq \
		| sort -V \
		| head -n 1

measure-stack-size-debug: build-debug
	@echo Enter Ctrl-A + X to exit
	$(QEMU_DEBUG) -d cpu,nochain 2>&1 \
		| rg sp \
		| sed 's/.*sp   \(.*\) gp.*/\1/g' \
		| sed 's/^0*//g' \
		| rg . \
		| uniq \
		| sort -V \
		| head -n 1

dump-dtb:
	$(QEMU) --machine dumpdtb=machine.dtb
	dtc -I dtb -O dts -o machine.dts machine.dtb

expand-%:
	cargo expand --target riscv64gc-unknown-none-elf $*

clean:
	rm -rf target/

build-riscv64:
	$(CARGO)

build-debug-riscv64:
	$(CARGO_DEBUG)

run-riscv64: build-riscv64 $(DRIVE)
	@echo Enter Ctrl-A + X to quit
	$(QEMU) $(QEMU_OPT)

run-debug-riscv64: build-debug-riscv64 $(DRIVE)
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

test-riscv64: $(DRIVE)
	$(CARGO) --test
	@echo Enter Ctrl-A + X to quit
	$(QEMU) $(QEMU_OPT)

test-debug-riscv64: $(DRIVE)
	$(CARGO_DEBUG) --test
	@echo Enter Ctrl-A + X to quit
	$(QEMU_DEBUG) $(QEMU_OPT)

objdump-riscv64:
	riscv64-unknown-linux-gnu-objdump -C $(OBJDUMP_OPT) $(KERNEL)

objdump-debug-riscv64:
	riscv64-unknown-linux-gnu-objdump -C $(OBJDUMP_OPT) $(KERNEL_DEBUG)

nm-riscv64:
	riscv64-unknown-linux-gnu-nm -C $(NM_OPT) $(KERNEL)

readelf-riscv64:
	riscv64-unknown-linux-gnu-readelf -C $(READELF_OPT) $(KERNEL)

strip-riscv64:
	riscv64-unknown-linux-gnu-strip -x $(KERNEL)

$(DRIVE):
	dd if=/dev/zero of=drive.bin bs=1M count=32
