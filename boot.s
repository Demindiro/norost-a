.section .init, "ax"
.global _start
_start:
    .cfi_startproc
    .cfi_undefined ra
    .option push
    .option norelax
	la	gp, __global_pointer$
	.option pop
	# Bandaid
	#auipc	sp, 0x4
	#la		s0, 0x10000
	#add		sp, sp, s0
	# FIXME fix whatever is breaking this
	la	sp, __stack_pointer
    add s0, sp, zero
	j	main
1:
	wfi
	j	1b
    .cfi_endproc
    .end
