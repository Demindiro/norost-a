TOOLCHAIN?=riscv64-unknown-linux-gnu
CC=$(TOOLCHAIN)-gcc
STRIP=$(TOOLCHAIN)-strip
READELF=$(TOOLCHAIN)-readelf
READELF_ARGS?=-a
OBJDUMP=$(TOOLCHAIN)-objdump
OBJDUMP_ARGS?=-S
ARCH?=riscv64

INCLUDE+= -nostartfiles -nostdlib -fno-builtin
INCLUDE+= -I../../../lib/c/dux/include/
INCLUDE+= -I../../../lib/c/posix/include/
INCLUDE+= -I../../../lib/c/kernel/include/
INCLUDE+= -I../../../lib/c/types/include/

SOURCES+= ../../../lib/c/posix/src/*.c
SOURCES+= ../../../lib/c/dux/src/*.c
INIT= ../../../lib/c/posix/src/crt0/riscv.s

build/init: hello.c | build/
	$(CC) -O3 $(INCLUDE) $(SOURCES) $(INIT) $< -o $@

build/:
	mkdir build/

readelf:
	$(READELF) $(READELF_ARGS) build/init

objdump:
	$(OBJDUMP) $(OBJDUMP_ARGS) build/init

clean:
	rm -rf build/
