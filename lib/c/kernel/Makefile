include ../../../common.mk


HEADERS  = $(shell find include -type f -name '*.h')


# Commands

default: install-headers

install-headers: $(patsubst %,$(SYSROOT)/%,$(HEADERS))


# Targets

$(SYSROOT)/include/%.h: include/%.h | $(HEADER_DIRS)
	cp $< $@
