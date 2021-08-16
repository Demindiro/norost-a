CROSS_COMPILE = riscv64-unknown-linux-gnu-
CC            = $(CROSS_COMPILE)gcc
NM            = $(CROSS_COMPILE)nm
OBJCOPY       = $(CROSS_COMPILE)objcopy
OBJDUMP       = $(CROSS_COMPILE)objdump
READELF       = $(CROSS_COMPILE)readelf
STRIP         = $(CROSS_COMPILE)strip
DB            = $(CROSS_COMPILE)gdb
RUST_TARGET   = riscv64gc-unknown-none-elf

mkfile_path := $(abspath $(lastword $(MAKEFILE_LIST)))
current_dir := $(dir $(mkfile_path))

QEMU=qemu-system-riscv64 \
		-s \
		-machine virt \
		-m 256M \
		-smp 1 \
		-bios $(FIRMWARE) \
		-kernel $(PWD)/../target/kernel.bin \
		-drive file=$(VIRTIO_DISK),format=raw,if=none,id=disk0 \
		-device virtio-blk-pci,drive=disk0

CARGO=cargo rustc \
	--release \
	--target $(RUST_TARGET) \
	$(CARGO_OPT) \
	-- \
	-C linker=$(CC) \
	-C link-arg=-nostartfiles \
	-C link-arg=-T$(current_dir)link.ld \
	-C link-arg=$(current_dir)start.s \
	-C no-redzone=yes


default: build

build:
	$(CARGO)

objdump:
	$(OBJDUMP) -C $(OBJDUMP_OPT) ../target/$(RUST_TARGET)/release/kernel

nm:
	$(NM) -C $(NM_OPT) $(KERNEL)

readelf:
	$(READELF) -C $(READELF_OPT) ../target/$(RUST_TARGET)/release/kernel

strip:
	$(STRIP) -x $(KERNEL)

list-target-features:
	rustc --target=$(RUST_TARGET) --print target-features
