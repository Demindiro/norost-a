OUTPUT = $(shell pwd)/output
TARGET = riscv64-dux-elf
BUILD  = build
PREFIX = $(OUTPUT)

JOBS ?= 4

BINUTILS_VERSION = 2.37
GCC_VERSION      = 11.1.0

GCC_PATH = $(OUTPUT)/bin:$(PATH)


default: gcc


install-dependencies-debian:
	sudo apt install \
		build-essential \
		bison flex \
		libgmp3-dev \
		libmpc-dev \
		libmpfr-dev \
		texinfo \
		libisl-dev
		#libcloog-isl-dev \


binutils: binutils-$(BINUTILS_VERSION) $(BUILD)/binutils
	cd $(BUILD)/binutils && \
		../../binutils-$(BINUTILS_VERSION)/configure \
		--target="$(TARGET)" \
		--prefix="$(PREFIX)" \
		--with-sysroot \
		--disable-nls \
		--disable-werror
	make -j$(JOBS) -C $(BUILD)/binutils
	make -j$(JOBS) -C $(BUILD)/binutils install

gcc: binutils gcc-$(GCC_VERSION) $(BUILD)/gcc
	cd $(BUILD)/gcc && \
		PATH="$(GCC_PATH)" ../../gcc-$(GCC_VERSION)/configure \
		--target="$(TARGET)" \
		--prefix="$(PREFIX)" \
		--disable-nls \
		--enable-languages=c \
		#--without-headers
	PATH="$(GCC_PATH)" make -j$(JOBS) -C $(BUILD)/gcc all-gcc
	PATH="$(GCC_PATH)" make -j$(JOBS) -C $(BUILD)/gcc all-target-libgcc
	PATH="$(GCC_PATH)" make -j$(JOBS) -C $(BUILD)/gcc install-gcc
	PATH="$(GCC_PATH)" make -j$(JOBS) -C $(BUILD)/gcc install-target-libgcc


binutils-$(BINUTILS_VERSION).tar.xz:
	curl https://ftp.gnu.org/gnu/binutils/binutils-$(BINUTILS_VERSION).tar.xz -O

gcc-$(GCC_VERSION).tar.xz:
	curl https://ftp.gnu.org/gnu/gcc/gcc-$(GCC_VERSION)/gcc-$(GCC_VERSION).tar.xz -O


binutils-$(BINUTILS_VERSION): binutils-$(BINUTILS_VERSION).tar.xz
	tar xJvf $<
	cd $@ && patch -p1 < ../patch/binutils-$(BINUTILS_VERSION).diff

gcc-$(GCC_VERSION): gcc-$(GCC_VERSION).tar.xz
	tar xJvf $<
	cd $@ && patch -p1 < ../patch/gcc-$(GCC_VERSION).diff

$(OUTPUT):
	mkdir -p $@

$(BUILD)/%:
	mkdir -p $@


create-patches:
	tar xJvf binutils-$(BINUTILS_VERSION).tar.xz -C /tmp
	diff -ru /tmp/binutils-$(BINUTILS_VERSION) binutils-$(BINUTILS_VERSION) | grep -v ^O > patch/binutils-$(BINUTILS_VERSION).diff
	diff -ruN /tmp/binutils-$(BINUTILS_VERSION)/ld/emulparams binutils-$(BINUTILS_VERSION)/ld/emulparams >> patch/binutils-$(BINUTILS_VERSION).diff || true
	rm -r /tmp/binutils-$(BINUTILS_VERSION)
	tar xJvf gcc-$(GCC_VERSION).tar.xz -C /tmp
	diff -ru /tmp/gcc-$(GCC_VERSION) gcc-$(GCC_VERSION) --exclude='configure' | grep -v ^O > patch/gcc-$(GCC_VERSION).diff
	diff -ruN /tmp/gcc-$(GCC_VERSION)/gcc/config gcc-$(GCC_VERSION)/gcc/config >> patch/gcc-$(GCC_VERSION).diff || true
	rm -r /tmp/gcc-$(GCC_VERSION)


clean: clean-build clean-unpacked clean-archives

clean-build:
	rm -rf $(BUILD)

clean-archives:
	rm -f gcc-$(GCC_VERSION).tar.xz
	rm -f binutils-$(BINUTILS_VERSION).tar.xz

clean-unpacked:
	rm -rf gcc-$(GCC_VERSION)
	rm -rf binutils-$(BINUTILS_VERSION)
