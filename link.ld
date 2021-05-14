OUTPUT_FORMAT("elf64-littleriscv")
OUTPUT_ARCH(riscv)

MEMORY
{
   RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 32M
}

STACK_SIZE = 64K;

ENTRY(_start)

SECTIONS
{
	.init : {
		_kernel_start = .;
		KEEP (*(SORT_NONE(.init)))
	}

	_text_start = .;
	.text : {
		*(.text)
	}
	_text_end = .;

	.data : {
		*(.data)
	}

	.rodata : {
		*(.rodata)
	}

	_global_pointer = . + 0x800;

	.got : {
		*(.got)
		*(.igot)
	}

	.bss : {
		*(.bss)
		*(COMMON)
	}

	.stack (NOLOAD): {
		. = ALIGN(8);
		_stack_start = .;
		. = . + STACK_SIZE;
		. = ALIGN(8);
		_stack_end = .;
		_stack_pointer = .;
	}

	_kernel_end = .;
}
