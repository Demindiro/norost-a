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

.section .text.hot
	.balign 4	# 0
interrupt_table:
	jal		zero, trap_handler	# User software interrupt _or_ instruction misaligned
	.balign 4	# 1
	j	mini_panic   # Supervisor software interrupt
	.balign 4	# 2
	j	mini_panic   # Reserved
	.balign 4	# 3
	j	mini_panic   # We shouldn't be able to catch machine interrupts
	.balign 4	# 4
	j	mini_panic   # User timer interrupt
	.balign 4	# 5
	j	mini_panic   # Supervisor timer interrupt
	.balign 4	# 6
	j	mini_panic   # Reserved
	.balign 4	# 7
	j	mini_panic   # We shouldn't be able to catch machine interrupts
	.balign 4	# 8
	j	mini_panic   # User external interrupt
	.balign 4	# 9
	j	sv_ext_int_handler   # Supervisor external interrupt
	.balign 4	# 10
	j	mini_panic   # Reserved
	.balign 4	# 11
	j	mini_panic   # We shouldn't be able to catch machine interrupts


sv_ext_int_handler:

	# Save some integer registers.
	csrrw	x31, sscratch, x31
	beqz	x31, mini_panic

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
	j	mini_panic	
	.balign 4	# 1
	j	mini_panic	
	.balign 4	# 2
	j	mini_panic	
	.balign 4	# 3
	j	mini_panic	
	.balign 4	# 4
	j	mini_panic	
	.balign 4	# 5
	j	mini_panic	
	.balign 4	# 6
	j	mini_panic
	.balign 4	# 7
	j	mini_panic	
	.balign 4	# 8
	jal		trap_syscall
	.balign 4	# 9
	j	mini_panic # S-mode shouldn't be performing S syscalls
	.balign 4	# 10
	j	mini_panic
	.balign 4	# 11
	j	mini_panic # We shouldn't be able to catch M-mode syscalls
	.balign 4	# 12
	j	mini_panic
	#ret
	.balign 4	# 13
	j	mini_panic
	.balign 4	# 14
	j	mini_panic
	.balign 4	# 15
	j	mini_panic

## Default handler for traps
trap_handler:
	# Save all integer registers. While we could just save the caller-saved registers, doing so
	# would risk an information leak and makes context switching a lot harder than it need be.
	csrrw	x31, sscratch, x31

	# Panic if sscratch is zero, which means we failed sometime after early boot
	beqz	x31, mini_panic

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
