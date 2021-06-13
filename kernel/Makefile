CROSS_COMPILE?=riscv64-unknown-linux-gnu-
CC=$(CROSS_COMPILE)gcc

DRIVE?=drive.bin

FIRMWARE?=../../riscv/opensbi/build/platform/generic/firmware/fw_jump.bin

#BOOT?=../../u-boot/u-boot.bin
#BOOT?=/home/david/Documents/u-boot/u-boot.bin

KERNEL?=../target/kernel
KERNEL_DEBUG?=../target/riscv64gc-unknown-none-elf/debug/kernel

IMG?=../target/dux.img

QEMU_OPT?=
QEMU?=qemu-system-riscv64 \
		-s \
		-machine virt \
		-nographic \
		-m 256M \
		-smp 1 \
		-bios $(FIRMWARE) \
		-kernel ../target/kernel.bin
		#-kernel $(BOOT) \
		#-drive file=$(IMG),format=raw,id=hd0 \
		#-device virtio-blk-device,drive=hd0
QEMU_DEBUG?=qemu-system-riscv64 \
		-s \
		-machine virt \
		-nographic \
		-m 256M \
		-smp 1 \
		-bios $(FIRMWARE) \
		-kernel ../target/kernel.bin
		#-drive file=$(DRIVE),format=raw

CARGO_OPT?=
CARGO?=cargo rustc \
	--release \
	--target riscv64gc-unknown-none-elf \
	$(CARGO_OPT) \
	-- \
	-C linker=riscv64-unknown-linux-gnu-gcc \
	-C link-arg=-nostartfiles \
	-C link-arg=-T$$PWD/link.ld \
	-C link-arg=$$PWD/boot.s \
	-C no-redzone=yes
CARGO_DEBUG?=cargo rustc \
	--target riscv64gc-unknown-none-elf \
	$(CARGO_OPT) \
	-- \
	-C linker=riscv64-unknown-linux-gnu-gcc \
	-C link-arg=-nostartfiles \
	-C link-arg=-T$$PWD/link.ld \
	-C link-arg=$$PWD/boot.s \
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

clean:
	cargo clean

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

build-riscv64:
	$(CARGO)
	fallocate -l 10MB $(IMG)

build-debug-riscv64:
	$(CARGO_DEBUG)

run-riscv64: build-riscv64 mkboot
	@echo Enter Ctrl-A + X to quit
	$(QEMU) $(QEMU_OPT)

run-debug-riscv64: build-debug-riscv64 mkboot-debug
	@echo Enter Ctrl-A + X to quit
	$(QEMU_DEBUG) $(QEMU_OPT)

gdb-riscv64:
	riscv64-unknown-linux-gnu-gdb \
		-ex='set arch riscv64' \
		-ex='target extended-remote localhost:1234' \
		../target/riscv64gc-unknown-none-elf/release/kernel

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
	riscv64-unknown-linux-gnu-objdump -C $(OBJDUMP_OPT) ../target/riscv64gc-unknown-none-elf/release/kernel

objdump-debug-riscv64:
	riscv64-unknown-linux-gnu-objdump -C $(OBJDUMP_OPT) $(KERNEL_DEBUG)

nm-riscv64:
	riscv64-unknown-linux-gnu-nm -C $(NM_OPT) $(KERNEL)

readelf-riscv64:
	riscv64-unknown-linux-gnu-readelf -C $(READELF_OPT) ../target/riscv64gc-unknown-none-elf/release/kernel

strip-riscv64:
	riscv64-unknown-linux-gnu-strip -x $(KERNEL)

gdb-run: build
	@echo Enter Ctrl-A + X to quit
	gdb --args $(QEMU) $(QEMU_OPT)

$(DRIVE):
	dd if=/dev/zero of=drive.bin bs=1M count=32

mkinit:
	cd ../services/init/hello_world && make clean && make
	touch src/main.rs

mkboot: build
	$(CC) -Wl,-Tsrc/boot/riscv.ld src/boot/riscv.s -o ../target/kernel -nostartfiles -nostdlib
	riscv64-unknown-linux-gnu-objcopy -O binary ../target/kernel ../target/kernel.bin

mkboot-debug: build-debug
	rm ../target/riscv64gc-unknown-none-elf/release/kernel
	mv ../target/riscv64gc-unknown-none-elf/debug/kernel ../target/riscv64gc-unknown-none-elf/release/kernel
	$(CC) -Wl,-Tsrc/boot/riscv.ld src/boot/riscv.s -o ../target/kernel -nostartfiles -nostdlib
	riscv64-unknown-linux-gnu-objcopy -O binary ../target/kernel ../target/kernel.bin

objdump-boot: mkboot
	riscv64-unknown-linux-gnu-objdump -C $(OBJDUMP_OPT) ../target/kernel

readelf-boot: mkboot
	riscv64-unknown-linux-gnu-readelf -C $(READELF_OPT) ../target/kernel

list-target-features:
	rustc --target=riscv64gc-unknown-none-elf --print target-features
