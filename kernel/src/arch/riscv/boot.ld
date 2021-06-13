OUTPUT_FORMAT("elf64-littleriscv")
OUTPUT_ARCH(riscv)

ENTRY(_start)

SECTIONS
{
	.text : {
		*(.text)
	}

	.bss : {
		*(.bss)
	}

	.data : {
		*(.data)
	}
}

