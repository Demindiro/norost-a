FIRMWARE    ?= ../riscv/opensbi/build/platform/generic/firmware/fw_jump.bin
KERNEL      ?= target/kernel.bin
VIRTIO_DISK ?= target/disk

QEMU=qemu-system-riscv64 \
		-s \
		-machine virt \
		-m 256M \
		-smp 1 \
		-bios $(FIRMWARE) \
		-kernel $(KERNEL) \
		-drive file=$(VIRTIO_DISK),format=raw,if=none,id=disk0 \
		-device virtio-blk-pci,drive=disk0 \
		-device virtio-gpu-pci \
		-serial stdio

dump-dtb:
	$(QEMU) --machine dumpdtb=/tmp/machine.dtb
	dtc -I dtb -O dts -o /tmp/machine.dts /tmp/machine.dtb

run: build $(VIRTIO_DISK)
	@echo Enter Ctrl-A + X to quit
	$(QEMU) $(QEMU_OPT)

RUST_TARGET ?= riscv64gc-unknown-none-elf

gdb: build $(VIRTIO_DISK)
	riscv64-unknown-linux-gnu-gdb \
		-ex='set arch riscv64' \
		-ex='target extended-remote localhost:1234' \
		target/$(RUST_TARGET)/release/kernel

gdb-run: build
	@echo Enter Ctrl-A + X to quit
	gdb --args $(QEMU) $(QEMU_OPT)

$(VIRTIO_DISK):
	fallocate -l $$((32 * 512)) $@
