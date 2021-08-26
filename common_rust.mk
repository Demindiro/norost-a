CARGO_BUILD = cargo build --release

readelf:
	$(READELF) -a ../../../target/riscv64gc-unknown-none-elf/release/$(NAME)

objdump:
	$(OBJDUMP) -SC ../../../target/riscv64gc-unknown-none-elf/release/$(NAME)

build:
	$(CARGO_BUILD)
