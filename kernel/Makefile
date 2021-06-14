include src/arch/riscv/Makefile

INIT        ?= init.elf
READELF_OPT ?= -a
VIRTIO_DISK ?= $(PWD)/../target/disk

mkdisk:
	fallocate -l $$((7 * 512)) $(VIRTIO_DISK)

clean:
	cargo clean
	rm -f machine.dts
	rm -f machine.dtb
