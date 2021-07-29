TOOLS_DIR = $(_common_mk_dir)thirdparty/tools/gcc/output/bin
TARGET    = riscv64-pc-dux
SYSROOT   = $(_common_mk_dir)sysroot/$(TARGET)

_common_mk_dir = $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
