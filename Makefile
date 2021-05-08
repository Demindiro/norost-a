default: build-riscv64

run: run-riscv64

clean:
	rm -r target/


build-riscv64:
	cargo rustc \
		--release \
		--target riscv64gc-unknown-none-elf \
		-- \
		-C linker=riscv64-linux-gnu-gcc \
		-C link-arg=-nostartfiles \
		-C link-arg=-Tlink.ld \

run-riscv64: build-riscv64
	@echo Enter Ctrl-A + X to quit
	qemu-system-riscv64 \
		-s \
		-machine virt \
		-nographic \
		-serial mon:stdio \
		-smp 1 \
		-m 32M \
		-bios none \
		-kernel target/riscv64gc-unknown-none-elf/release/dux

dump:
	riscv64-linux-gnu-objdump -SC target/riscv64-unknown-none-elf/release/dux
