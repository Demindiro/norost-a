default: build


test:
	#make -C lib/c/std/ test
	make -C services/init/b0
	make -C . run

include run.mk


build:
	$(MAKE) -C kernel
	$(MAKE) -C boot

clean:
	rm -rf target/

init:
	$(MAKE) -C services/init/multi_process
