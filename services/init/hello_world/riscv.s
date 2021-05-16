.globl _start

.section .text, "ax"

_start:
	li		a7, 1				# write
	la		a0, hello_world
	li		a1, 13				# length
#ecall
	li		a1, 300 * 1000 * 1000
1:
	addi	a1, a1, -1
	bne		zero, a1, 1b
	ret

.section .rodata, "a"

hello_world:
	.ascii		"Hello, world!"
	.byte		0xa0			# newline
hello_world_end:
