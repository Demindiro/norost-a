TOOLS_DIR = $(_common_mk_dir)thirdparty/tools/gcc/output/bin
TARGET    = riscv64-pc-dux
SYSROOT   = $(_common_mk_dir)sysroot/$(TARGET)

CARGO_TARGET     = riscv64gc-unknown-none-elf
CARGO_OUTPUT_DIR = $(_common_mk_dir)target/$(CARGO_TARGET)/release
CARGO_PROFILE    = release
export CARGO_BUILD_TARGET = riscv64gc-unknown-none-elf

CC        = $(TOOLS_DIR)/$(TARGET)-gcc
AR        = $(TOOLS_DIR)/$(TARGET)-ar
AS        = $(TOOLS_DIR)/$(TARGET)-as
STRIP     = $(TOOLS_DIR)/$(TARGET)-strip
READELF   = $(TOOLS_DIR)/$(TARGET)-readelf
OBJDUMP   = $(TOOLS_DIR)/$(TARGET)-objdump

_common_mk_dir = $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

default: build
