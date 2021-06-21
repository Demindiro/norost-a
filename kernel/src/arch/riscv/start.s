.globl __stack_pointer
.globl _start

.section .initelf, "a"
	# Align to page boundary
	.align	12

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

	# Set the stack pointer
	la		sp, _stack_end
    add		s0, sp, zero
	# Set ra to zero to indicate end of call stack
	mv		ra, zero
	call	main

1:
	wfi
	j	1b
    .cfi_endproc
