## crt0 used when linking against this library only.

.globl _start

.equ	FILE_UUID, 0
.equ	FILE_POSITION, (FILE_UUID + 16)
.equ	FILE_ADDRESS, (FILE_POSITION + 8)

.set	__files_list, 0x87650000
# Same thing, no errors
.equ	__FILES_LIST, 0x87650000

.section .text
_start:
    .cfi_startproc
    .cfi_undefined ra
    .option push
    .option norelax
	la		gp, __global_pointer$
	.option pop

	## Set up stdin, stdout and stderr.
	# the stack pointer is set to the very top of the stack
	# Amount of entries
	ld		s1, -8(sp)
	# Store entry
	la		a0, __files_count
	sd		s1, 0(a0)
	# Adjust for element count
	addi	sp, sp, -8
	# Multiply by 8 + 16 (64-bit address + 128 bit UUID)
	slli	a1, s1, 3
	slli	s1, s1, 4
	add		s1, a1, a1
	sub		s1, sp, s1

	# Load the address with files and map a page in it.
	# TODO account for the actual amount of memory needed.
	#la		s0, __files_list
	li		s0, __FILES_LIST
	li		a7, 3			# mem_alloc
	mv		a0, s0			# address
	li		a1, 1			# page count
	li		a2, 0b11		# flags (WR)
	ecall
	# TODO add some way to abort
0:
	bnez	a0, 0b

	# Jump to the loop check
	j		1f
0:
	ld		t0, -8(sp)		# address
	ld		t1, -16(sp)		# uuid (low)
	ld		t2, -24(sp)		# uuid (high)
	sd		t0, FILE_ADDRESS (s0)
	sd		t1, FILE_UUID + 0 (s0)
	sd		t2, FILE_UUID + 8 (s0)

	# Go to next element
	addi	sp, sp, -24
	addi	s0, s0, 24
1:
	# Repeat if we haven't reached the end
	bne		s1, sp, 0b

	## Load the argument count & pointer & make all the argument strings
	## zero-terminated.
	ld		a0, -8(sp)
	ld		a1, -16(sp)
	addi	sp, sp, -8
	# Multiply by 8
	slli	s0, a0, 3
	add		s0, sp, s0

	# Iterate all strings
	j		1f
0:
	ld		t0, -8(sp)
	lh		t1, 0(s1)

	# Shift the string to the left by two bytes
2:
	# Since strings must be aligned on a 2 byte boundary we can safely use
	# lh/sh
	lh		t2, 2(t0)
	sh		t2, 0(t0)
	addi	t1, t1, -2
	bnez	t1, 2b

	# Put a null terminator
	sb		zero, 0(t0)

	addi	sp, sp, -8
1:
	# Repeat if we haven't reached the end
	bne		s0, sp, 0b

	## Set return address to zero to indicate end of call stack
	addi	sp, sp, -8
	sd		zero, 0(sp)

	## Initialize libraries
	call	__dux_init
	call	__posix_init

	## Run main
	call	main

	## Exit (TODO)
0:
	wfi
	j		0b
    .cfi_endproc
