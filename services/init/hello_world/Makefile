TOOLCHAIN?=riscv64-unknown-linux-gnu
CC=$(TOOLCHAIN)-gcc
STRIP=$(TOOLCHAIN)-strip
READELF=$(TOOLCHAIN)-readelf
READELF_ARGS?=-a
OBJDUMP=$(TOOLCHAIN)-objdump
OBJDUMP_ARGS?=-S

build/riscv64: riscv.s | build/ 
	$(CC) -nostartfiles -nostdlib $< -o $@
	$(STRIP) -R .eh_frame -R .eh_frame_hdr build/riscv64

build/:
	mkdir build/

readelf:
	$(READELF) $(READELF_ARGS) build/riscv64

objdump:
	$(OBJDUMP) $(OBJDUMP_ARGS) build/riscv64

clean:
	rm -rf build/
