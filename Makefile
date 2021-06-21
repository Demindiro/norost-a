include run.mk

default: build


build:
	$(MAKE) -C kernel
	$(MAKE) -C boot

clean:
	rm -rf target/
