include ../../../common.mk

READELF_ARGS?=-a
OBJDUMP_ARGS?=-S

build/init: hello.c | build/
	$(CC) -O3 $< -o $@

build/:
	mkdir build/

readelf:
	$(READELF) $(READELF_ARGS) build/init

objdump:
	$(OBJDUMP) $(OBJDUMP_ARGS) build/init

clean:
	rm -rf build/

symlink:
	rm -f ../../../init.elf
	ln -s services/init/hello_world/build/init ../../../init.elf
