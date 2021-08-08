include ../../../common.mk

NAME       = libdux.a
CARGO_NAME = libdux_ar.a
OUTPUT = $(SYSROOT)/lib/$(NAME)

build:
	cargo build --release
	make -C . "$(OUTPUT)"

$(OUTPUT): $(CARGO_OUTPUT_DIR)/$(CARGO_NAME)
	mv $(CARGO_OUTPUT_DIR)/$(CARGO_NAME) $(OUTPUT)
