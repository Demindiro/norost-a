TOOLS_DIR ?= ../../../thirdparty/tools/gcc/output/bin
PREFIX    ?= riscv64-pc-dux
TARGET    ?= $(PREFIX)

INCLUDE  = -Iinclude
INCLUDE += -I../dux/include
INCLUDE += -I../kernel/include
INCLUDE += -I../errno/include

CC_ARGS   = -nostartfiles -nostdlib -ffreestanding -O3 -fPIC
CC        = $(TOOLS_DIR)/$(PREFIX)-gcc
AR        = $(TOOLS_DIR)/$(PREFIX)-ar
AS        = $(TOOLS_DIR)/$(PREFIX)-as

HEADERS  = $(shell find include -type f -name '*.h')
SOURCES  = $(shell find src -type f -name '*.c')
OBJECTS  = $(shell for f in `find src -type f -name '*.c' -printf "%P\n"`; do echo "$(BUILD)/$${f%.c}.o"; done)
DIRECTORIES = $(shell for n in `find src -type d -printf "%P\n"`; do if [ $$n ]; then echo "$(BUILD)/$$n"; fi; done)

OUTPUT = output/$(PREFIX)
BUILD  = build/$(PREFIX)

STATIC_LIBS = ../dux/output/dux.a

build: $(OUTPUT)/crt0.o $(OUTPUT)/posix.a

$(OUTPUT)/crt0.o: src/crt0/$(PREFIX).s | $(OUTPUT)
	$(AS) $< -o $@

$(OUTPUT)/posix.a: $(OBJECTS) $(STATIC_LIBS) | $(OUTPUT)
	$(AR) rcT $@ $^
	printf 'create $@\naddlib $@\nsave\nend' | $(AR) -M

$(BUILD)/%.o: src/%.c | $(DIRECTORIES)
	$(CC) -c $< -o $@ $(INCLUDE) $(CC_ARGS)

../dux/output/dux.a:
	make -C ../dux

$(BUILD):
	mkdir -p $@

$(OUTPUT):
	mkdir -p $@

$(DIRECTORIES):
	mkdir -p $@

format:
	for f in $(HEADERS) $(SOURCES); do indent -linux -par "$$f"; done

clean:
	for f in $(HEADERS) $(SOURCES); do rm -f "$$f~"; done
	rm -rf $(BUILD)

clean-all: clean
	rm -rf $(OUTPUT)
