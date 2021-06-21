CROSS_COMPILE = riscv64-unknown-linux-gnu-
CC            = $(CROSS_COMPILE)gcc
NM            = $(CROSS_COMPILE)nm
OBJCOPY       = $(CROSS_COMPILE)objcopy
OBJDUMP       = $(CROSS_COMPILE)objdump
READELF       = $(CROSS_COMPILE)readelf
STRIP         = $(CROSS_COMPILE)strip
DB            = $(CROSS_COMPILE)gdb

mkfile_path := $(abspath $(lastword $(MAKEFILE_LIST)))
current_dir := $(dir $(mkfile_path))

default: build

build:
	$(CC) -Wl,-T$(current_dir)boot.ld $(current_dir)boot.s -o $(PWD)/target/kernel -nostartfiles -nostdlib
	$(OBJCOPY) -O binary $(PWD)/target/kernel ../target/kernel.bin

nm:
	$(NM) -C $(NM_OPT) $(KERNEL)

strip:
	$(STRIP) -x $(KERNEL)

objdump:
	$(OBJDUMP) -C $(OBJDUMP_OPT) ../target/kernel

readelf:
	$(READELF) -C $(READELF_OPT) ../target/kernel
