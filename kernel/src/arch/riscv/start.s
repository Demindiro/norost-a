.globl __stack_pointer
.globl _start

.section .initelf, "a"
	# Align to page boundary
	.align	12
initelf:
	.incbin	"kernel/init.elf"
	.equ	INITELF_LEN,	. - initelf
initelf_size:
	.quad	INITELF_LEN

.section .init, "ax"
kernel_offset:
#t 	.quad	KERNEL_OFFSET + (1f & 0x7ffff)
_start:
    .cfi_startproc
    .cfi_undefined ra

	# Set the global pointer
    .option push
    .option norelax
	la		gp, _global_pointer
	.option pop

	li		a7, 0x01
	li		a6, 0
	li		t0, 10
	li		a0, 0xa
	ecall

	# Set the stack pointer
	la		sp, _stack_end
    add		s0, sp, zero
	# Set ra to zero to indicate end of call stack
	mv		ra, zero
	# Set pointer and length to initfs
	la		a2, initelf
	#li		a3, INITELF_LEN
	la		a3, initelf_size
	ld		a3, 0(a3)
	call	main

1:
	wfi
	j	1b
    .cfi_endproc
