include run.mk

default: build


build:
	$(MAKE) -C kernel
	$(MAKE) -C boot

clean:
	rm -rf target/

init:
	$(MAKE) -C services/init/hello_world_virtio
