readelf:
	$(READELF) -a ../../../target/riscv64gc-unknown-none-elf/release/$(NAME)

objdump:
	$(OBJDUMP) -SC ../../../target/riscv64gc-unknown-none-elf/release/$(NAME)
