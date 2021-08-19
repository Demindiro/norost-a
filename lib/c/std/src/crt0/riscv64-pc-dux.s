## crt0 used when linking against this library only.

.globl _start

## Definition of FILE in stdio.h at the time of writing:
# typedef struct {
#	kernel_uuid_t _uuid;
# 	uint64_t _position;
#	pid_t _address;
#	const char *_path;
# } FILE;

.struct	0


.equ	FILE_UUID_SIZE, 16
.equ	FILE_POSITION_SIZE, 8
.equ	FILE_ADDRESS_SIZE, 8
.equ	FILE_PATH_SIZE, 8

.equ	FILE_UUID, 0
.equ	FILE_POSITION, (FILE_UUID + FILE_UUID_SIZE)
.equ	FILE_ADDRESS, (FILE_POSITION + FILE_POSITION_SIZE)
.equ	FILE_PATH, (FILE_ADDRESS + FILE_ADDRESS_SIZE)

.equ	SIZEOF_FILE, (FILE_PATH + FILE_PATH_SIZE)

# The actual address of the files list that will be stored in __files_list
.equ	__FILES_LIST, 0x87650000

.section .text
_start:
    .cfi_startproc
    .cfi_undefined ra
    .option push
    .option norelax
	la		gp, __global_pointer$
	.option pop


	## Load the argument count & pointer & make all the argument strings
	## zero-terminated.
	ld		s3, -8(sp)
	addi	s4, sp, -16 + 8
	addi	sp, sp, -8
	# Multiply by 8
	slli	s0, s3, 3
	sub		s4, s4, s0
	sub		s0, sp, s0

	# Iterate all strings
	j		1f
0:
	ld		t0, -8(sp)
	lh		t1, 0(t0)
	add		t1, t0, t1

	# Shift the string to the left by two bytes
	j		3f
2:
	# Since strings must be aligned on a 2 byte boundary we can safely use
	# lh/sh
	lh		t2, 2(t0)
	sh		t2, 0(t0)
	addi	t0, t0, 2
3:
	blt		t0, t1, 2b

	# Put a null terminator
	sb		zero, 0(t1)

	addi	sp, sp, -8
1:
	# Repeat if we haven't reached the end
	bne		s0, sp, 0b


	## Set up stdin (0), stdout (1), stderr (2) and cwd (3)
	# the stack pointer is set to the very top of the stack
	# Amount of entries
	ld		s1, -8(sp)
	# Store entry
	la		t5, __files_count
	sd		s1, 0(t5)
	# Adjust for element count
	addi	sp, sp, -8

	# Don't try to copy or alias anything if stdin is not defined
	beqz	s1, 3f

	# Backup s1 in t3 as we need it later
	mv		t3, s1
	# Multiply by 8 + 16 (64-bit address + 128 bit UUID) & determine
	# end address of stack pointer
	slli	a1, s1, 3
	slli	s1, s1, 4
	add		s1, s1, a1
	sub		s1, sp, s1

	# Load the address with files and map a page in it.
	# TODO account for the actual amount of memory needed.
	li		s0, __FILES_LIST
	li		a7, 3			# mem_alloc
	mv		a0, s0			# address
	li		a1, 1			# page count
	li		a2, 0b11		# flags (WR)
	ecall
	# TODO add some way to abort
0:
	bnez	a0, 0b

	# Set the address to the files list.
	la		a0, __files_list
	sd		s0, 0(a0)

	# Backup s0 in t4 as we need it later
	mv		t4, s0

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
	addi	s0, s0, SIZEOF_FILE
1:
	# Repeat if we haven't reached the end
	bne		s1, sp, 0b

	# Alias stdin to stdout if it is not defined
	addi	t0, t3, -1
	bgtz	t0, 0f
	ld		t0, 0 * SIZEOF_FILE + FILE_ADDRESS (t4)
	ld		t1, 0 * SIZEOF_FILE + FILE_UUID + 0 (t4)
	ld		t2, 0 * SIZEOF_FILE + FILE_UUID + 8 (t4)
	sd		t0, 1 * SIZEOF_FILE + FILE_ADDRESS (t4)
	sd		t1, 1 * SIZEOF_FILE + FILE_UUID + 0 (t4)
	sd		t2, 1 * SIZEOF_FILE + FILE_UUID + 8 (t4)
0:

	# Alias stdout to stderr if it is not defined
	addi	t0, t3, -2
	bgtz	t0, 0f
	ld		t0, 1 * SIZEOF_FILE + FILE_ADDRESS (t4)
	ld		t1, 1 * SIZEOF_FILE + FILE_UUID + 0 (t4)
	ld		t2, 1 * SIZEOF_FILE + FILE_UUID + 8 (t4)
	sd		t0, 2 * SIZEOF_FILE + FILE_ADDRESS (t4)
	sd		t1, 2 * SIZEOF_FILE + FILE_UUID + 0 (t4)
	sd		t2, 2 * SIZEOF_FILE + FILE_UUID + 8 (t4)
0:

	# Clear cwd (fill with -1) if it is not defined
	addi	t0, t3, -3
	bgtz	t0, 0f
	li		t0, -1
	sd		t0, 3 * SIZEOF_FILE + FILE_ADDRESS (t4)
	sd		t0, 3 * SIZEOF_FILE + FILE_UUID + 0 (t4)
	sd		t0, 3 * SIZEOF_FILE + FILE_UUID + 8 (t4)
0:

3:

	## Set return address to zero to indicate end of call stack
	addi	sp, sp, -8
	sd		zero, 0(sp)

	## Initialize libraries
	call	__dux_init
	call	__posix_init

	## Run main
	mv		a0, s3
	mv		a1, s4
	call	main

	## Exit (TODO)
0:
	wfi
	j		0b
    .cfi_endproc
