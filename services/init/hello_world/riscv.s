.globl _start

.section .text, "ax"

_start:
	addi	sp, sp, -8
	li		a7, 1				# write syscall
	li		a0, 42				# File descriptor (unused atm)
	la		a1, hello_world		# Pointer to buffer
	li		a2, 14				# Buffer size
	sd		ra, 0(sp)
	ecall
	ld		ra, 0(sp)
	addi	sp, sp, 8
	ret

.section .rodata, "a"

hello_world:
	.ascii		"Hello, world!"
	.byte		0x0a			# newline
hello_world_end:
