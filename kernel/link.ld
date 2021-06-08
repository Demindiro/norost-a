OUTPUT_FORMAT("elf64-littleriscv")
OUTPUT_ARCH(riscv)

STACK_SIZE = 4K;

ENTRY(_start)

KERNEL_START = -64K;

SECTIONS
{
	. = KERNEL_START;
	.init : {
		_kernel_start = .;
		KEEP (*(SORT_NONE(.init)))
	}

	_text_start = .;
	.text : {
		*(.text .text.*)
	}
	_text_end = .;

	.data : {
		*(.data .data.*)
	}

	.rodata : {
		*(.rodata .rodata.*)
	}

	_global_pointer = . + 0x800;

	.got : {
		*(.got)
		*(.igot)
	}

	.bss : {
		*(.bss .bss.*)
		*(COMMON)
	}

	.stack : {
		. = ALIGN(8);
		_stack_start = .;
		. = . + STACK_SIZE;
		. = ALIGN(8);
		_stack_end = .;
		_stack_pointer = .;
	}

	.initobject : {
		*(.initobject)
	}

	.initfs : {
		*(.initfs)
	}

	_kernel_end = .;

	.symtab : {
		*(.symtab)
	}

	.psymtab : {
		*(.psymtab)
	}
}
