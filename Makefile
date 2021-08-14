default: build


test:
	#make -C lib/c/std/ test
	make -C lib/rust/dux_ar
	make -C lib/c/std/ -B
	make -C services/init/minish/ -B
	make -C services/driver/uart -B
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

format:
	cargo fmt --all
