BUILD = build
OUTPUT = output

CC = ../../tools/gcc/output/bin/riscv64-unknown-elf-gcc

OBJECTS = $(shell for n in `find functions -type f -name '*.c' -printf "%P\n"`; do echo "$(BUILD)/$${n%.c}.o"; done)
DIRECTORIES = $(shell for n in `find functions -type d -printf "%P\n"`; do if [ $$n ]; then echo "$(BUILD)/$$n"; fi; done)
INCLUDE = -Iinclude -Iinclude/pdclib
DEFINES = -D_PDCLIB_BUILD -D_PDCLIB_STATIC_DEFINE
CC_ARGS = -O3 -flto


default: $(OUTPUT)/libc.so

$(OUTPUT)/libc.so: $(OBJECTS) $(OUTPUT)
	$(CC) $(OBJECTS) -o $@ -nostdlib $(CC_ARGS)

$(BUILD)/%.o: functions/%.c | $(DIRECTORIES)
	@echo $@: $<
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
