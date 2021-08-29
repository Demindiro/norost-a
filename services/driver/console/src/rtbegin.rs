global_asm!(
	"
	.globl	_start
	_start:

		# Set return address to 0 to aid debugger
		addi	sp, sp, -8
		sd		zero, 0(sp)

		call	main

		# Loop forever as we can't exit
	0:
		j		0b
	",
);
