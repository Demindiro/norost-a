.globl _start


.section .rodata, "a"

	.align		12
running:
	.ascii		"Running!"
	.byte		0x0a			# newline
	.equ		RUNNING_LEN,		. - running

	.align		12
hello_world:
	.ascii		"Hello, world!"
	.byte		0x0a			# newline
	.equ		HELLO_WORLD_LEN,	. - hello_world


.section .bss, "aw"

	.align		12
request_queue:
	.fill		4096, 1, 0

	.align		12
completion_queue:
	.fill		4096, 1, 0


.section .text, "ax"

_start:
	# Set kernel pointer to request and completion queue
	li		a7, 1					# io_set_client_buffers
	la		a0, request_queue		# request buffer
	li		a1, 0					# size of request buffer (2^0 == 1)
	la		a2, completion_queue	# completion buffer
	li		a3, 0					# size of completion buffer (2^0 == 1)
	ecall

	# Write start message
	la		a0, hello_world
	li		a1, HELLO_WORLD_LEN
	call	puts

	call	test_alloc_memory
	call	test_alloc_shared_memory

0:
	j		0b

	# Exit TODO
	li		a7, 2				# exit syscall
	li		a0, 0				# exit code
	ecall


.equ	MEM_ALLOC_ADDR,	0x1230000

test_alloc_memory:

	# Allocate one memory page.
	li		a7,	3				# mem_alloc
	li		a0, MEM_ALLOC_ADDR	# address
	li		a1, 1				# amount of pages
	li		a2, 0b011			# flags (RW)
	ecall

	# Write the alphabet to the page.
	li		t0, 'A'
	li		t1, 26
	mv		t2, a1
0:
	sd		t0, 0(t2)
	addi	t0, t0, 1
	addi	t1, t1, -1
	addi	t2, t2, 1
	blt		zero, t1, 0b

	# Append a newline
	li		t0, 0xa
	sd		t0, 0(t2)

	# Print the alphabet
	mv		a0, a1
	li		a1, 27
	mv		s0, ra
	call	puts
	mv		ra, s0

	ret


.equ	MEM_ALLOC_ADDR,	0x3210000

test_alloc_shared_memory:

	# Allocate one memory page.
	li		a7,	3				# mem_alloc_shared
	li		a0, MEM_ALLOC_ADDR	# address
	li		a1, 1				# amount of pages
	li		a2, 0b1011			# flags (SHAREABLE + RW)
	ecall

	# Write the alphabet to the page.
	li		t0, 'a'
	li		t1, 26
	mv		t2, a1
0:
	sd		t0, 0(t2)
	addi	t0, t0, 1
	addi	t1, t1, -1
	addi	t2, t2, 1
	blt		zero, t1, 0b

	# Append a newline
	li		t0, 0xa
	sd		t0, 0(t2)

	# Print the alphabet
	mv		a0, a1
	li		a1, 27
	mv		s0, ra
	call	puts
	mv		ra, s0

	ret

## Write a string to stdout
##
## Arguments:
## * a0: the string. Must be page-aligned and written on a shareable page.
## * a1: the length of the string
puts:
	la		t0, request_queue
	sb		zero, 1(t0)				# priority
	sh		zero, 2(t0)				# flags
	sw		zero, 4(t0)				# file handle
	sd		zero, 8(t0)				# offset
	sd		a0, 16(t0)				# page
	sd		a1, 24(t0)				# length
	li		t1, 0xdeadbeef
	sd		t1, 32(t0)				# userdata
	# Ensure the rest of the data is written out before writing the opcode.
	fence
	li		t1, 2					# WRITE
	sb		t1, 0(t0)				# write opcode

	# Sync IO
	li		a7, 0				# io_wait
	mv		a0, zero			# flags
	ecall

	ret
