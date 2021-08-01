global_asm!(
	"
	.globl	_start
	_start:
		li		a7, 3				# mem_alloc
		li		a0, 0xffff0000		# address
		li		a1, 0x10000 / 4096	# size (64K)
		li		a2, 0b011			# flags (RW)
		ecall

	0:
		bnez	a0, 0b				# Loop forever on error

		li		sp, 0xffffffff			# Set stack pointer

		addi	sp, sp, -8			# Set return address to 0 to aid debugger
		sd		zero, 0(sp)

		call	main

	0:
		j		0b				# Loop forever as we can't exit
	",
);
