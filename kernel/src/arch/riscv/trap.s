.line 0
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

	# Save some registers so we can get to work (a0-2 == x10-12)
	load_gp_regs	10, 12, x31

	# Set sepc to that of the notification handler.
	gp_load		a0, 2 * GP_REGBYTES + REGSTATE_SIZE, x31
	csrw		sepc, a0

	# Some real fucking hacky shit to figure out how the fuck these motherfucking interrupts work.

	# Enable SUM
	csrr	a0, sstatus
	li		a1, 1 << 18
	or		a0, a0, a1
	csrw	sstatus, a0

	# Claim shit and set value argument
	li		a0, 0x4 * 0x10000 * 0x10000 # The base address of the shit we mapped
	li		a1, 0x20 * 0x10000 + 0x4 # The offset of the claim shit
	add		a0, a0, a1				# Goto claim shit
	li		a1, 0x1000 # Context stride
	add		a0, a0, a1 # Add context stride shit
	lw		a1, 0(a0) # Claim the source. Not doing this causes a loop.
	#sw		a1, 0(a0) # Pretend we completed shit

	# Disable SUM
	csrr	a2, sstatus
	li		a0, ~(1 << 18)
	and		a2, a2, a0
	csrw	sstatus, a2

	# Set type argument (0 == external interrupt)
	mv		a0, zero
	
	# Restore some registers
	#save_gp_regs	10, 12, x31
	csrrw	x31, sscratch, x31

	# Jump to notification handler
	# FIXME check for the right TID dumbass.
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

	# Save registers x1 - x30
	save_gp_regs	1, 30, x31

	# Save program counter
	# We increase the counter by 4 bytes as ecall is also 4 bytes long
	# & we don't want to execute it again.
	csrr	t0, sepc
	addi	t0, t0, 4
	sd		t0, 0 * GP_REGBYTES (x31)

	# Set pointer to task struct argument
	mv		a6, x31

	# An untested attempt at catering to pipelining
	# Rereading this later, I am very sorry for trying to be a smartass, again
	la		x28, sync_trap_table		# jmp
	ld		sp, REGSTATE_SIZE + 0 * GP_REGBYTES (x31)		# stack pointer
	csrr	x29, scause					# jmp
	sd		x30, 30 * GP_REGBYTES (x31)	# store A
	slli	x29, x29, 2					# jmp

	# Store the task's x31 register properly
	# Use of the x30 register is intentional, as we still need x31 to point to
	# the register storage
	csrrw	x30, sscratch, x31			# store B	(restores original sscratch)
	add		x28, x28, x29				# jmp
	sd		x30, 31 * GP_REGBYTES (x31)	# store B

	# Execute the appropriate routine
	jalr	ra, x28						# jmp

	# Restore all integer registers
	csrr		x31, sscratch
	load_gp_regs	1, 31, x31

	# Continue in userspace
	sret

# Handler for syscalls.
trap_syscall:

	# FIXME we shouldn't have to do this ever.
	la		s0, global_mapping
	ld		s0, 0(s0)
	csrrw	s0, satp, s0
	sfence.vma

	addi	sp, sp, -1 * GP_REGBYTES

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
	slli	a7, a7, GP_REGORDER
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
	load_gp_regs	1, 9, x31
	# x10 == a0 and x11 == a1, so skip
	load_gp_regs	12, 31, x31
	sret

	# Since erroneous syscalls are presumably rare, put the error handling
	# near the end to improve efficiency somewhat
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
	ld		t0, REGSTATE_SIZE + 1 * GP_REGBYTES (a0)
	csrw	satp, t0
	sfence.vma
	# Setup the scratch register.
	csrw	sscratch, a0
	# Restore the program counter
	ld		t0, 0 * GP_REGBYTES (a0)
	csrw	sepc, t0
	# Restore all float registers
	load_fp_regs	a0
	# Restore all integer registers
	load_gp_regs	1, 9, a0
	# a0 == x10, so skip until end
	load_gp_regs	11, 31, a0
	load_gp_regs	10, 10, a0
	sret
