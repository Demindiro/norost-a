## Interrupt handler designed to work with PLICs

.equ	PLIC_STRIDE_CONTEXT	,	0x1000
.equ	PLIC_CLAIM_OFFSET	,	(0x20 * 0x10000 + 0x4)

external_interrupt_handler:

	j mini_panic # Temporary while learning about MSI-X

	# Save _all_ the general purpose registers.
	# This isn't strictly necessary when the interrupt is addressed at the
	# active task, but a branch to compare the TID is likely more expensive
	# anyways.
	csrrw			x31, sscratch, x31
	save_gp_regs	1, 30, x31
	# Save the remaining x31 register and restore sscratch as well.
	csrrw			x30, sscratch, x31
	gp_store		x30, 31 * GP_REGBYTES, x31
	# Save pc
	csrr			x30, sepc
	gp_store		x30, 0 * GP_REGBYTES, x31

	# Fix kernel stack, needed for call later
	# FIXME this causes UB with the pseudo task, as it has no valid stack
	# pointer
	gp_load			sp, TASK_STACK, x31

	# Claim the interrupt.
	# We need to do this now because we can't return to userspace otherwise.

	# Load PLIC base address
	la		a0, plic_address			# TODO should be a constant, ideally.
	gp_load	a0, 0, a0

	# Offset to claim base address
	li		a1, PLIC_CLAIM_OFFSET
	add		a0, a0, a1

	# Offset by context
	li		a1, PLIC_STRIDE_CONTEXT
	li		a2, 1 # FIXME context ID stub
	mul		a1, a1, a2
	add		a0, a0, a1

	# Claim the source
	# s0 is used as we need to preserve it across a call.
	lw		s0, 0(a0)

	# Figure out which task to send a notification to.
	la		a0, plic_reservations
	li		a2, GP_REGBYTES
	mul		a2, a2, s0
	add		a0, a0, a2
	# Use -GP_REGBYTES as the table doesn't include source 0
	# FIXME this also needs to be atomic I suppose?
	gp_load	a0, -GP_REGBYTES, a0

	# FIXME this may fail under normal circumstances; it is possible the
	# interrupt handler is running while the driver unregisters an interrupt
	#
	# Unlikely, but certainly possible.
	addi	a2, a0, 1
	beqz	a2, mini_panic

	# Get a pointer to the task with the given address
	call	executor_get_task

	# Put the interrupt source in a1, as intended
	mv		a1, s0

	# FIXME don't panic you doof
	beqz	a0, mini_panic

	# We need a0 to be in x31 regardless, so just do it now.
	mv		x31, a0

	# Update sscratch for the trap handler
	csrw	sscratch, x31

	# Load the task's VMS
	gp_load	a0, TASK_VMS, x31
	csrw	satp, a0
	sfence.vma

	# Set the IRQ field.
	# FIXME this needs to be atomic
	sh		a1, TASK_IRQ (x31)

	# Set sepc to that of the notification handler
	gp_load		a0, 2 * GP_REGBYTES + REGSTATE_SIZE, x31
	csrw		sepc, a0

	# Restore stack
	load_gp_regs 2, 2, x31

	# Enable SUM
	li		a2, 1 << 18
	csrs	sstatus, a2

	# Push original a[017] and pc to stack
	gp_load		x30, 10 * GP_REGBYTES, x31
	gp_store	x30, -4 * GP_REGBYTES, sp
	gp_load		x30, 11 * GP_REGBYTES, x31
	gp_store	x30, -3 * GP_REGBYTES, sp
	gp_load		x30, 17 * GP_REGBYTES, x31
	gp_store	x30, -2 * GP_REGBYTES, sp
	gp_load		x30,  0 * GP_REGBYTES, x31
	gp_store	x30, -1 * GP_REGBYTES, sp

	# Disable SUM and SPP to ensure we will enter usermode
	ori		a2, a2, 1 << 8
	csrc	sstatus, a2

	# Set address, which is -1 for the kernel
	li		a7, -1
	# Set type (0 == external interrupt)
	li		a0, 0

	# Load all registers except the stack pointer (x2), since
	# the stack pointer is already loaded, and a[017] (x10/11/17).
	load_gp_regs 1, 1, x31
	load_gp_regs 3, 9, x31
	load_gp_regs 12, 16, x31
	load_gp_regs 18, 31, x31

	# == FIXME save the FP registers
	li		t0, 1 << 13
	csrc	sstatus, t0
	li		t0, 1 << 14
	csrs	sstatus, t0
	# ==

	# Jump to notification handler
	sret
