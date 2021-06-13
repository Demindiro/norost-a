CROSS_COMPILE = riscv64-unknown-linux-gnu-
CC            = $(CROSS_COMPILE)gcc
NM            = $(CROSS_COMPILE)nm
OBJCOPY       = $(CROSS_COMPILE)objcopy
OBJDUMP       = $(CROSS_COMPILE)objdump
READELF       = $(CROSS_COMPILE)readelf
STRIP         = $(CROSS_COMPILE)strip
DB            = $(CROSS_COMPILE)gdb
RUST_TARGET   = riscv64gc-unknown-none-elf

FIRMWARE?=../../riscv/opensbi/build/platform/generic/firmware/fw_jump.bin

mkfile_path := $(abspath $(lastword $(MAKEFILE_LIST)))
current_dir := $(dir $(mkfile_path))

QEMU=qemu-system-riscv64 \
		-s \
		-machine virt \
		-nographic \
		-m 256M \
		-smp 1 \
		-bios $(FIRMWARE) \
		-kernel $(PWD)/../target/kernel.bin

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

dump-dtb:
	$(QEMU) --machine dumpdtb=$(OUTPUT)/machine.dtb
	dtc -I dtb -O dts -o $(OUTPUT)/machine.dts $(OUTPUT)/machine.dtb

build:
	$(CARGO)

run: build mkboot
	@echo Enter Ctrl-A + X to quit
	$(QEMU) $(QEMU_OPT)

gdb:
	riscv64-unknown-linux-gnu-gdb \
		-ex='set arch riscv64' \
		-ex='target extended-remote localhost:1234' \
		../target/$(RUST_TARGET)/release/kernel

test:
	$(CARGO) --test
	@echo Enter Ctrl-A + X to quit
	$(QEMU) $(QEMU_OPT)

objdump:
	$(OBJDUMP) -C $(OBJDUMP_OPT) $(OUTPUT)

nm:
	$(NM) -C $(NM_OPT) $(KERNEL)

readelf:
	$(READELF) -C $(READELF_OPT) $(OUTPUT)

strip:
	$(STRIP) -x $(KERNEL)

gdb-run: build
	@echo Enter Ctrl-A + X to quit
	gdb --args $(QEMU) $(QEMU_OPT)

mkinit:
	cd ../services/init/hello_world && make clean && make
	touch src/main.rs

mkboot: build
	$(CC) -Wl,-T$(current_dir)boot.ld $(current_dir)boot.s -o $(PWD)/../target/kernel -nostartfiles -nostdlib
	$(OBJCOPY) -O binary $(PWD)/../target/kernel ../target/kernel.bin

objdump-boot: mkboot
	$(OBJDUMP) -C $(OBJDUMP_OPT) ../target/kernel

readelf-boot: mkboot
	$(READELF) -C $(READELF_OPT) ../target/kernel

list-target-features:
	rustc --target=$(RUST_TARGET) --print target-features
