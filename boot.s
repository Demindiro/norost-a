.section .init, "ax"
.global _start
_start:
    .cfi_startproc
    .cfi_undefined ra
    .option push
    .option norelax
	la	gp, __global_pointer$
	.option pop
	la	sp, __stack_pointer
    add s0, sp, zero
	# Set ra to zero to indicate end of call stack
	mv		ra, zero
	call	main
1:
	wfi
	j	1b
    .cfi_endproc
    .end
