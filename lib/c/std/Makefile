include ../../../common.mk

INCLUDE  = -Iinclude
INCLUDE += -I../dux/include
INCLUDE += -I../kernel/include
INCLUDE += -I../errno/include

CC_ARGS   = -nostartfiles -nostdlib -ffreestanding -O3 -fPIC -Wall
CC        = $(TOOLS_DIR)/$(TARGET)-gcc
AR        = $(TOOLS_DIR)/$(TARGET)-ar
AS        = $(TOOLS_DIR)/$(TARGET)-as

HEADERS  = $(shell find include -type f -name '*.h')
SOURCES  = $(shell find src -type f -name '*.c')
OBJECTS  = $(shell for f in `find src -type f -name '*.c' -printf "%P\n"`; do echo "$(BUILD)/$${f%.c}.o"; done)
DIRECTORIES = $(shell for n in `find src -type d -printf "%P\n"`; do if [ $$n ]; then echo "$(BUILD)/$$n"; fi; done)

HEADER_DIRS = $(patsubst %,$(OUTPUT)/%,$(shell for f in $(HEADERS); do echo "$${f%/*}"; done | sort | uniq))

OUTPUT = $(SYSROOT)
BUILD  = build/$(TARGET)

STATIC_LIBS = ../dux/output/dux.a



# Commands

default: install-headers build

build: $(OUTPUT)/lib/crt0.o $(OUTPUT)/lib/libc.a

install-headers: $(patsubst %,$(OUTPUT)/%,$(HEADERS))

format:
	for f in $(HEADERS) $(SOURCES); do indent -linux -par "$$f"; done

clean:
	for f in $(HEADERS) $(SOURCES); do rm -f "$$f~"; done
	rm -rf $(BUILD)


# Targets

$(OUTPUT)/lib/crt0.o: src/crt0/$(TARGET).s | $(OUTPUT)/lib
	$(AS) $< -o $@

$(OUTPUT)/lib/libc.a: $(OBJECTS) | $(STATIC_LIBS) $(OUTPUT)/lib
	rm -f $@
	$(AR) rc $@ $^
	printf 'create $@\naddlib $@\naddlib $(STATIC_LIBS)\nsave\nend' | $(AR) -M

$(OUTPUT)/include/%.h: include/%.h | $(HEADER_DIRS)
	cp $< $@

$(BUILD)/%.o: src/%.c | $(DIRECTORIES)
	$(CC) -c $< -o $@ $(INCLUDE) $(CC_ARGS)

../dux/output/dux.a:
	make -C ../dux

$(BUILD) $(DIRECTORIES) $(HEADER_DIRS):
	mkdir -p $@
