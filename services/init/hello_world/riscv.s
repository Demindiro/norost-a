.globl _start


.section .rodata, "a"

running:
	.ascii		"Running!"
	.byte		0x0a			# newline
.equ	RUNNING_LEN,		. - running

hello_world:
	.ascii		"Hello, world!"
	.byte		0x0a			# newline
.equ	HELLO_WORLD_LEN,	. - hello_world


.section .text, "ax"

_start:
	# Write start message
	li		a7, 1				# write syscall
	li		a0, 42				# File descriptor (unused atm)
	la		a1, running			# Pointer to buffer
	li		a2, RUNNING_LEN		# Buffer size
	ecall

	# Get TID
	li		a7, 4				# task_id syscall
	ecall
	mv		t1, a1

	# Write hello world 4 times, with the length being shortened by the task ID
	li		t0, 4
0:
	# Write
	li		a7, 1				# write syscall
	li		a0, 42				# File descriptor (unused atm)
	la		a1, hello_world		# Pointer to buffer
	li		a2, HELLO_WORLD_LEN	# Buffer size
	add		a1, a1, t1
	sub		a2, a2, t1
	ecall

	# Sleep / yield
	li		a7, 3				# sleep syscall
	li		a0, 0				# seconds
	li		a1, 0				# nanoseconds
	ecall

	# Loop
	addi	t0, t0, -1
	blt		zero, t0, 0b

	# Exit
	li		a7, 2				# exit syscall
	li		a0, 0				# exit code
	ecall
