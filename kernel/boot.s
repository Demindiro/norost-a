.globl __stack_pointer
.globl _start

.section .init, "ax"
_start:
    .cfi_startproc
    .cfi_undefined ra
    .option push
    .option norelax
	la	gp, _global_pointer
	.option pop
	la	sp, _stack_end
    add s0, sp, zero
	# Set ra to zero to indicate end of call stack
	mv		ra, zero
	call	main
1:
	wfi
	j	1b
    .cfi_endproc
    .end
