global_asm!(
	"
	.globl	_start
	_start:

		# Set return address to 0 to aid debugger
		sd		zero, -8(sp)
		addi	sp, sp, -8

		call	main

		# Loop forever as we can't exit
	0:
		j		0b

	66:	# Abort (TODO)
		j		66b
	",
);
