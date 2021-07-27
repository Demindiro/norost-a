default: build


include run.mk


build:
	$(MAKE) -C kernel
	$(MAKE) -C boot

clean:
	rm -rf target/

init:
	$(MAKE) -C services/init/multi_process
