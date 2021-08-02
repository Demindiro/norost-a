include ../../../common.mk

NAME       = minish
OUTPUT_DIR = $(SYSROOT)/bin
OUTPUT     = $(OUTPUT_DIR)/$(NAME)

CC_ARGS = -static -O3 -Wall

# Commands

build: $(OUTPUT)

symlink:
	rm -f ../../../init.elf
	ln -s sysroot/$(TARGET)/bin/minish ../../../init.elf

# Targets

$(OUTPUT): src/main.c | $(OUTPUT_DIR) $(SYSROOT)/lib/libc.a
	$(CC) $^ -o $(OUTPUT) $(CC_ARGS)

$(OUTPUT_DIR):
	mkdir -p $@
