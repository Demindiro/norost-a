default: build


test:
	#make -C lib/c/std/ test
	make -C services/driver/mouse
	make -C services/driver/virtio_input
	make -C services/driver/console
	make -C services/driver/virtio_gpu
	make -C services/driver/fat
	make -C lib/rust/dux_ar
	make -C lib/c/std/ -B
	make -C services/init/minish/ -B
	make -C services/driver/plic
	make -C services/driver/uart
	make -C services/driver/virtio_block
	make -C services/driver/pci
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
