## Trap handling routines
##
## These are implemented separately due to naked functions being too inflexible and cumbersome.

#.include	"registers.s"

.equ		SYSCALL_MAX,			16
.equ		SYSCALL_ERR_NOCALL, 	1

.globl trap_handler
.globl trap_init
.globl trap_stop_task
.globl trap_start_task

.section .data
global_mapping:
	.quad	0
early_panic_msg:
	.asciz	"Early panic!"
scause_msg:
	.asciz	"scause  0x"
sepc_msg:
	.asciz	"sepc    0x"
stval_msg:
	.asciz	"stval   0x"

.section .text

trap_section_start:

	.balign 4	# 0
interrupt_table:
	jal		zero, trap_handler	# User software interrupt _or_ instruction misaligned
	.balign 4	# 1
	j	trap_early_handler   # Supervisor software interrupt
	.balign 4	# 2
	j	trap_early_handler   # Reserved
	.balign 4	# 3
	j	trap_early_handler   # We shouldn't be able to catch machine interrupts
	.balign 4	# 4
	j	trap_early_handler   # User timer interrupt
	.balign 4	# 5
	j	trap_early_handler   # Supervisor timer interrupt
	.balign 4	# 6
	j	trap_early_handler   # Reserved
	.balign 4	# 7
	j	trap_early_handler   # We shouldn't be able to catch machine interrupts
	.balign 4	# 8
	j	trap_early_handler   # User external interrupt
	.balign 4	# 9
	j	sv_ext_int_handler   # Supervisor external interrupt
	.balign 4	# 10
	j	trap_early_handler   # Reserved
	.balign 4	# 11
	j	trap_early_handler   # We shouldn't be able to catch machine interrupts


sv_ext_int_handler:

	# Save some integer registers.
	csrrw	x31, sscratch, x31
	beqz	x31, trap_early_handler

	# Save registers
	sd		x1, 1 * REGBYTES (x31)
	sd		x2, 2 * REGBYTES (x31)

	# Some real fucking hacky shit to figure out how the fuck these motherfucking interrupts work.

	# Enable SUM
	csrr	x1, sstatus
	li		x2, 1 << 18
	or		x1, x1, x2
	csrw	sstatus, x1

	# Claim shit
	li		x1, 0x4 * 0x10000 * 0x10000 # The base address of the shit we mapped
	li		x2, 0x20 * 0x10000 + 0x4 # The offset of the claim shit
	add		x1, x1, x2				# Goto claim shit
	li		x2, 0x1000 # Context stride
	add		x1, x1, x2 # Add context stride shit
	lw		x2, 0(x1) # Claim the source. Not doing this causes a loop.
	#sw		x2, 0(x1) # Pretend we completed shit

	# Disable SUM
	csrr	x1, sstatus
	li		x2, ~(1 << 18)
	and		x1, x1, x2
	csrw	sstatus, x1
	
	# Restore registers
	ld		x1, 1 * REGBYTES (x31)
	ld		x2, 2 * REGBYTES (x31)
	csrrw	x31, sscratch, x31

	sret

	.balign 4	# 0
sync_trap_table:
	j	trap_early_handler	
	.balign 4	# 1
	j	trap_early_handler	
	.balign 4	# 2
	j	trap_early_handler	
	.balign 4	# 3
	j	trap_early_handler	
	.balign 4	# 4
	j	trap_early_handler	
	.balign 4	# 5
	j	trap_early_handler	
	.balign 4	# 6
	j	trap_early_handler
	.balign 4	# 7
	j	trap_early_handler	
	.balign 4	# 8
	jal		trap_syscall
	.balign 4	# 9
	j	trap_early_handler # S-mode shouldn't be performing S syscalls
	.balign 4	# 10
	j	trap_early_handler
	.balign 4	# 11
	j	trap_early_handler # We shouldn't be able to catch M-mode syscalls
	.balign 4	# 12
	j	trap_early_handler
	#ret
	.balign 4	# 13
	j	trap_early_handler
	.balign 4	# 14
	j	trap_early_handler
	.balign 4	# 15
	j	trap_early_handler

## Default handler for traps
trap_handler:
	# Save all integer registers. While we could just save the caller-saved registers, doing so
	# would risk an information leak and makes context switching a lot harder than it need be.
	csrrw	x31, sscratch, x31

	# Panic if sscratch is zero, which means we failed sometime after early boot
	beqz	x31, trap_early_handler

	# Save registers
	sd		x1, 1 * REGBYTES (x31)
	sd		x2, 2 * REGBYTES (x31)
	sd		x3, 3 * REGBYTES (x31)
	sd		x4, 4 * REGBYTES (x31)
	sd		x5, 5 * REGBYTES (x31)
	sd		x6, 6 * REGBYTES (x31)
	sd		x7, 7 * REGBYTES (x31)
	sd		x8, 8 * REGBYTES (x31)
	sd		x9, 9 * REGBYTES (x31)
	sd		x10, 10 * REGBYTES (x31)
	sd		x11, 11 * REGBYTES (x31)
	sd		x12, 12 * REGBYTES (x31)
	sd		x13, 13 * REGBYTES (x31)
	sd		x14, 14 * REGBYTES (x31)
	sd		x15, 15 * REGBYTES (x31)
	sd		x16, 16 * REGBYTES (x31)
	sd		x17, 17 * REGBYTES (x31)
	sd		x18, 18 * REGBYTES (x31)
	sd		x19, 19 * REGBYTES (x31)
	sd		x20, 20 * REGBYTES (x31)
	sd		x20, 20 * REGBYTES (x31)
	sd		x20, 20 * REGBYTES (x31)
	sd		x21, 21 * REGBYTES (x31)
	sd		x22, 22 * REGBYTES (x31)
	sd		x23, 23 * REGBYTES (x31)
	sd		x24, 24 * REGBYTES (x31)
	sd		x25, 25 * REGBYTES (x31)
	sd		x26, 26 * REGBYTES (x31)
	sd		x27, 27 * REGBYTES (x31)
	sd		x28, 28 * REGBYTES (x31)
	sd		x29, 29 * REGBYTES (x31)
	# Save program counter
	# We increase the counter by 4 bytes as ecall is also 4 bytes long
	# & we don't want to execute it again.
	csrr	t0, sepc
	addi	t0, t0, 4
	sd		t0, 0 * REGBYTES (x31)

	# Set pointer to task struct argument
	mv		a6, x31
	# An untested attempt at catering to pipelining
	la		x28, sync_trap_table		# jmp
	ld		sp, REGSTATE_SIZE + 0 * REGBYTES (x31)		# stack pointer
	csrr	x29, scause					# jmp
	sd		x30, 30 * REGBYTES (x31)	# store A
	slli	x29, x29, 2					# jmp
	csrrw	x30, sscratch, x31			# store B	(restores original mscratch)
	add		x28, x28, x29				# jmp
	sd		x30, 31 * REGBYTES (x31)	# store B
	# Execute the appropriate routine
	jalr	ra, x28						# jmp
	# Restore all integer registers
	csrr	x31, sscratch
	ld		x1, 1 * REGBYTES (x31)
	ld		x2, 2 * REGBYTES (x31)
	ld		x3, 3 * REGBYTES (x31)
	ld		x4, 4 * REGBYTES (x31)
	ld		x5, 5 * REGBYTES (x31)
	ld		x6, 6 * REGBYTES (x31)
	ld		x7, 7 * REGBYTES (x31)
	ld		x8, 8 * REGBYTES (x31)
	ld		x9, 9 * REGBYTES (x31)
	ld		x10, 10 * REGBYTES (x31)
	ld		x11, 11 * REGBYTES (x31)
	ld		x12, 12 * REGBYTES (x31)
	ld		x13, 13 * REGBYTES (x31)
	ld		x14, 14 * REGBYTES (x31)
	ld		x15, 15 * REGBYTES (x31)
	ld		x16, 16 * REGBYTES (x31)
	ld		x17, 17 * REGBYTES (x31)
	ld		x18, 18 * REGBYTES (x31)
	ld		x19, 19 * REGBYTES (x31)
	ld		x20, 20 * REGBYTES (x31)
	ld		x20, 20 * REGBYTES (x31)
	ld		x20, 20 * REGBYTES (x31)
	ld		x21, 21 * REGBYTES (x31)
	ld		x22, 22 * REGBYTES (x31)
	ld		x23, 23 * REGBYTES (x31)
	ld		x24, 24 * REGBYTES (x31)
	ld		x25, 25 * REGBYTES (x31)
	ld		x26, 26 * REGBYTES (x31)
	ld		x27, 27 * REGBYTES (x31)
	ld		x28, 28 * REGBYTES (x31)
	ld		x29, 29 * REGBYTES (x31)
	ld		x30, 30 * REGBYTES (x31)
	ld		x31, 31 * REGBYTES (x31)
	sret

# Handler for syscalls.
trap_syscall:

	# FIXME we shouldn't have to do this ever.
	la		s0, global_mapping
	ld		s0, 0(s0)
	csrrw	s0, satp, s0
	sfence.vma

	addi	sp, sp, -1 * REGBYTES

	# Skip the ecall instruction, which is always 4 bytes long (there is no
	# compressed version of it).
	csrr	t0, sepc
	addi	t0, t0, 4
	csrw	sepc, t0

	# Check if the syscall exists, otherwise return the 'no syscall' error code
	li		t1, SYSCALL_MAX
	bgeu	a7, t1, 1f

	# Look up the entry in the call table
	la		t0, syscall_table
	slli	a7, a7, REGORDER
	add		t0, t0, a7
	ld		t0, 0(t0)

	# Perform the call
	jalr	ra, t0

	# FIXME we shouldn't have to do this ever.
	csrw	satp, s0
	sfence.vma

	# Restore all integer registers except a0 and a1, then return
0:
	csrr	x31, sscratch
	ld		x1, 1 * REGBYTES (x31)
	ld		x2, 2 * REGBYTES (x31)
	ld		x3, 3 * REGBYTES (x31)
	ld		x4, 4 * REGBYTES (x31)
	ld		x5, 5 * REGBYTES (x31)
	ld		x6, 6 * REGBYTES (x31)
	ld		x7, 7 * REGBYTES (x31)
	ld		x8, 8 * REGBYTES (x31)
	ld		x9, 9 * REGBYTES (x31)
	# x10 == a0 and x11 == a1, so skip
	ld		x12, 12 * REGBYTES (x31)
	ld		x13, 13 * REGBYTES (x31)
	ld		x14, 14 * REGBYTES (x31)
	ld		x15, 15 * REGBYTES (x31)
	ld		x16, 16 * REGBYTES (x31)
	ld		x17, 17 * REGBYTES (x31)
	ld		x18, 18 * REGBYTES (x31)
	ld		x19, 19 * REGBYTES (x31)
	ld		x20, 20 * REGBYTES (x31)
	ld		x20, 20 * REGBYTES (x31)
	ld		x20, 20 * REGBYTES (x31)
	ld		x21, 21 * REGBYTES (x31)
	ld		x22, 22 * REGBYTES (x31)
	ld		x23, 23 * REGBYTES (x31)
	ld		x24, 24 * REGBYTES (x31)
	ld		x25, 25 * REGBYTES (x31)
	ld		x26, 26 * REGBYTES (x31)
	ld		x27, 27 * REGBYTES (x31)
	ld		x28, 28 * REGBYTES (x31)
	ld		x29, 29 * REGBYTES (x31)
	ld		x30, 30 * REGBYTES (x31)
	ld		x31, 31 * REGBYTES (x31)
	sret

1:
	li		a0, SYSCALL_ERR_NOCALL
	j		0b


## Initialize the trap CSR and the interrupt table
trap_init:
	la		t0, interrupt_table
	ori		t0, t0, 1
	csrw	stvec, t0
	ret

## Early boot trap handler. It avoids any memory accesses outside the kernel
## image and enters a halt loop immediately. It also prints the scause, sepc
## and stval registers.
.align	2
trap_early_handler:

	# Print panic message
	la		s0, early_panic_msg
	call	trap_print_msg
	call	trap_print_lf

	# Print scause
	la		s0, scause_msg
	call	trap_print_msg
	csrr	s0, scause
	call	trap_print_num

	# Print sepc
	la		s0, sepc_msg
	call	trap_print_msg
	csrr	s0, sepc
	call	trap_print_num

	# Print stval
	la		s0, stval_msg
	call	trap_print_msg
	csrr	s0, stval
	call	trap_print_num

	# Halt forever
0:
	wfi
	j		0b

	# String message printing routine
	# s0: message
trap_print_msg:
	lb		a0, 0(s0)
0:
	li		a7, 1
	li		a6, 0
	ecall
	addi	s0, s0, 1
	lb		a0, 0(s0)
	bnez	a0, 0b
	ret

	# Hexadecimal number printing routine
	# Always 16 digits.
	# Also prints newline
	# s0: number
trap_print_num:
	li		s1, 60
	li		s2, 10
0:
	srl		a0, s0, s1
	andi	a0, a0, 0xf
	blt		a0, s2, 1f
	addi	a0, a0, 'a' - 10 - '0'
1:
	addi	a0, a0, '0'
	li		a7, 1
	li		a6, 0
	ecall
	addi	s1, s1, -4
	bgez	s1, 0b

	# Newline printing routine
trap_print_lf:
3:
	li		a7, 1
	li		a6, 0
	li		a0, 0xa
	ecall
	ret


## Setup trap handler for early init. The early trap handler
## will halt immediately for ease of debugging.
trap_early_init:
	la		t0, trap_early_handler
	csrw	stvec, t0
	ret


## Load the task's saved registers, then start running the task using mret.
##
## Arguments:
## - a0: A pointer to the task structure.
trap_start_task:
	# FIXME we should do this in init (or never, rather)
	csrr	t0, satp
	la		t1, global_mapping
	sd		t0, 0(t1)
	# Switch to U-mode when executing mret.
	li		t0, 0 << 11
	csrw	sstatus, t0
	# Set up the VMS.
	ld		t0, REGSTATE_SIZE + 1 * REGBYTES (a0)
	csrw	satp, t0
	sfence.vma
	# Setup the scratch register.
	csrw	sscratch, a0
	# Restore the program counter
	ld		t0, 0 * REGBYTES (a0)
	csrw	sepc, t0
	# Restore all float registers
	load_float_registers	a0
	# Restore all integer registers
	ld		x1, 1 * REGBYTES (a0)
	ld		x2, 2 * REGBYTES (a0)
	ld		x3, 3 * REGBYTES (a0)
	ld		x4, 4 * REGBYTES (a0)
	ld		x5, 5 * REGBYTES (a0)
	ld		x6, 6 * REGBYTES (a0)
	ld		x7, 7 * REGBYTES (a0)
	ld		x8, 8 * REGBYTES (a0)
	ld		x9, 9 * REGBYTES (a0)
	# a0 == x10, so skip
	ld		x11, 11 * REGBYTES (a0)
	ld		x12, 12 * REGBYTES (a0)
	ld		x13, 13 * REGBYTES (a0)
	ld		x14, 14 * REGBYTES (a0)
	ld		x15, 15 * REGBYTES (a0)
	ld		x16, 16 * REGBYTES (a0)
	ld		x17, 17 * REGBYTES (a0)
	ld		x18, 18 * REGBYTES (a0)
	ld		x19, 19 * REGBYTES (a0)
	ld		x20, 20 * REGBYTES (a0)
	ld		x20, 20 * REGBYTES (a0)
	ld		x20, 20 * REGBYTES (a0)
	ld		x21, 21 * REGBYTES (a0)
	ld		x22, 22 * REGBYTES (a0)
	ld		x23, 23 * REGBYTES (a0)
	ld		x24, 24 * REGBYTES (a0)
	ld		x25, 25 * REGBYTES (a0)
	ld		x26, 26 * REGBYTES (a0)
	ld		x27, 27 * REGBYTES (a0)
	ld		x28, 28 * REGBYTES (a0)
	ld		x29, 29 * REGBYTES (a0)
	ld		x30, 30 * REGBYTES (a0)
	ld		x31, 31 * REGBYTES (a0)
	ld		a0, 10 * REGBYTES (a0)
	sret

trap_section_end:

#.if		trap_section_end - trap_section_start <= 4096
#	# OK
#.else
#	.err	"Trap section covers more than one page"
#.abort
#.endif
#.if		(trap_section_start & ~0xfff) - (trap_section_end  & ~0xfff) == 0
#	# OK
#.else
#	.err	"Trap section crosses page boundaries"
#.abort
#.endif
