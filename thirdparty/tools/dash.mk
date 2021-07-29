TOOLS = ../../tools/gcc/output/bin
TARGET = riscv64-dux-elf
CC = $(TOOLS)/$(TARGET)-gcc

conf: gen
	cd dash && CC=$(CC) ./configure

gen:
	cd dash && CC=$(CC) ./autogen.sh
