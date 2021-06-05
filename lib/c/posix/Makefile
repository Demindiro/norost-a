TOP := $(firstword $(MAKEFILE_LIST))
INCLUDE+= -I../kernel/ $(TOP)/src/common.c $(TOP)/src/crt0/riscv.s
CC_ARGS+= -nostartfiles -nostdlib
CC?=gcc
