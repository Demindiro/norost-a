BUILD = build
OUTPUT = output

TARGET = riscv64-pc-dux

CC = ../../tools/gcc/output/bin/$(TARGET)-gcc
AR = ../../tools/gcc/output/bin/$(TARGET)-ar

OBJECTS = $(shell for n in `find functions -type f -name '*.c' -printf "%P\n"`; do echo "$(BUILD)/$${n%.c}.o"; done)
DIRECTORIES = $(shell for n in `find functions -type d -printf "%P\n"`; do if [ $$n ]; then echo "$(BUILD)/$$n"; fi; done)
INCLUDE = -Iinclude -Iinclude/pdclib
DEFINES = -D_PDCLIB_BUILD
#-D_PDCLIB_STATIC_DEFINE
CC_ARGS = -O3 -shared -nostdlib -ffreestanding -fPIC

STATIC_LIBS = ../../../lib/c/posix/output/$(TARGET)/posix.a

default: static
	
static: $(OUTPUT)/c.a

shared: $(OUTPUT)/c.so

$(OUTPUT)/c.so: $(OBJECTS) $(STATIC_LIBS) | $(OUTPUT)
	$(CC) $^ -o $@ $(CC_ARGS)

$(OUTPUT)/c.a: $(OBJECTS) $(STATIC_LIBS) | $(OUTPUT)
	$(AR) rcT $@ $^
	printf 'create $@\naddlib $@\nsave\nend' | $(AR) -M

$(BUILD)/%.o: functions/%.c | $(DIRECTORIES)
	$(CC) -c $< -o $@ $(INCLUDE) -nostdlib $(CC_ARGS)

$(BUILD):
	mkdir $@

$(OUTPUT):
	mkdir $@

$(DIRECTORIES):
	mkdir -p $@

clean:
	rm -rf $(BUILD)

clean-all: clean
	rm -rf $(OUTPUT)
