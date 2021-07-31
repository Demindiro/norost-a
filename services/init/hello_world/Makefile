include ../../../common.mk

NAME       = hello_world
OUTPUT_DIR = $(SYSROOT)/bin
OUTPUT     = $(OUTPUT_DIR)/$(NAME)

READELF_ARGS?=-a
OBJDUMP_ARGS?=-S

$(OUTPUT): hello.c | $(OUTPUT_DIR)
	$(CC) -O3 $< -o $@

$(OUTPUT_DIR):
	mkdir -p $@

readelf:
	$(READELF) $(READELF_ARGS) $(OUTPUT)

objdump:
	$(OBJDUMP) $(OBJDUMP_ARGS) $(OUTPUT)

clean:
	rm -rf build/

symlink:
	rm -f ../../../init.elf
	ln -s services/init/hello_world/build/init ../../../init.elf
