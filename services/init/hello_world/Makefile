TOOLCHAIN?=riscv64-unknown-linux-gnu
CC=$(TOOLCHAIN)-gcc
STRIP=$(TOOLCHAIN)-strip
READELF=$(TOOLCHAIN)-readelf
READELF_ARGS?=-a
OBJDUMP=$(TOOLCHAIN)-objdump
OBJDUMP_ARGS?=-S
ARCH?=riscv64

build/init: $(ARCH).s | build/
	$(CC) -nostartfiles -nostdlib $< -o $@
	$(STRIP) -R .eh_frame -R .eh_frame_hdr $@

build/:
	mkdir build/

readelf:
	$(READELF) $(READELF_ARGS) build/init

objdump:
	$(OBJDUMP) $(OBJDUMP_ARGS) build/init

clean:
	rm -rf build/
