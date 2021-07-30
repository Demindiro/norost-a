TOOLS_DIR = $(_common_mk_dir)thirdparty/tools/gcc/output/bin
TARGET    = riscv64-pc-dux
SYSROOT   = $(_common_mk_dir)sysroot/$(TARGET)

CC        = $(TOOLS_DIR)/$(TARGET)-gcc
AR        = $(TOOLS_DIR)/$(TARGET)-ar
AS        = $(TOOLS_DIR)/$(TARGET)-as
STRIP     = $(TOOLS_DIR)/$(TARGET)-strip
READELF   = $(TOOLS_DIR)/$(TARGET)-readelf
OBJDUMP   = $(TOOLS_DIR)/$(TARGET)-objdump

_common_mk_dir = $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
