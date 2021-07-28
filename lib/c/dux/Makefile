TOOLS_DIR ?= ../../../thirdparty/tools/gcc/output/bin
PREFIX    ?= riscv64-unknown-elf

INCLUDE  = -Iinclude
INCLUDE += -I../kernel/include

CC_ARGS   = -nostartfiles -nostdlib -ffreestanding -O3
CC        = $(TOOLS_DIR)/$(PREFIX)-gcc
AR        = $(TOOLS_DIR)/$(PREFIX)-ar

HEADERS  = $(shell find include -type f -name '*.h')
SOURCES  = $(shell find src -type f -name '*.c')
OBJECTS  = $(shell for f in `find src -type f -name '*.c' -printf "%P\n"`; do echo "$(BUILD)/$${f%.c}.o"; done)
DIRECTORIES = $(shell for n in `find src -type d -printf "%P\n"`; do if [ $$n ]; then echo "$(BUILD)/$$n"; fi; done)

OUTPUT = output
BUILD  = build


$(OUTPUT)/dux.a: $(OBJECTS) | $(OUTPUT)
	$(AR) rcs $@ $^

$(BUILD)/%.o: src/%.c | $(DIRECTORIES) $(BUILD)
	$(CC) -c $< -o $@ $(INCLUDE) $(CC_ARGS) $(DEFINES)

$(BUILD):
	mkdir $@

$(OUTPUT):
	mkdir $@

$(DIRECTORIES):
	mkdir -p $@

format:
	for f in $(HEADERS) $(SOURCES); do indent -linux -par "$$f"; done

clean:
	for f in $(HEADERS) $(SOURCES); do rm -f "$$f~"; done
	rm -rf $(BUILD)
