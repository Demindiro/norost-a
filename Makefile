default: build


build: build-kernel


build-kernel:
	$(MAKE) -C kernel


clean:
	rm -rf target/


run:
	$(MAKE) -C kernel $@
