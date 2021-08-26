include ../../../common.mk
include ../../../common_rust.mk

NAME = console_driver

build: font.bitmap
	$(CARGO_BUILD)

font.bitmap: font.bitmap.xz
	test -e $@ || xz -dk $<
