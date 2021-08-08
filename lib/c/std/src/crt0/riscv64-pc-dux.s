## crt0 used when linking against this library only.

.globl _start

# Reserve 64 Kb stack space to start with
.equ STACK_TOP_ADDRESS, 0x0fff * 0x10000
.equ STACK_INITIAL_SIZE, 16 # 64 KiB
.equ PROT_RW, 0b011

.section .text
_start:
    .cfi_startproc
    .cfi_undefined ra
    .option push
    .option norelax
	la		gp, __global_pointer$
	.option pop
	
	# Allocate pages for stack frame
	li		a7, 3		# mem_alloc
	li		a0, STACK_TOP_ADDRESS - STACK_INITIAL_SIZE * 0x1000
	li		a1, STACK_INITIAL_SIZE
	li		a2, PROT_RW
	ecall
	# FIXME handle error properly
	blt		zero, a0, 0f

	li		sp, STACK_TOP_ADDRESS - 8
	# Set return address to zero to indicate end of call stack
	sd		zero, 0(sp)

	# Initialize libraries
	call	__dux_init
	call	__posix_init

	# Run main
	call	main

	# Exit (TODO)
0:
	wfi
	j		0b
    .cfi_endproc
