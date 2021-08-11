## Interrupt handler designed to work with PLICs

.equ	PLIC_STRIDE_CONTEXT	,	0x1000
.equ	PLIC_CLAIM_OFFSET	,	(0x20 * 0x10000 + 0x4)

external_interrupt_handler:

	# Save _all_ the general purpose registers.
	# This isn't strictly necessary when the interrupt is addressed at the
	# active task, but a branch to compare the TID is likely more expensive
	# anyways.
	csrrw			x31, sscratch, x31
	save_gp_regs	1, 30, x31
	# Save the remaining x31 register and restore sscratch as well.
	csrrw			x30, sscratch, x31
	gp_store		x30, 31 * GP_REGBYTES, x31

	# Set sepc to that of the notification handler.
	gp_load		a0, 2 * GP_REGBYTES + REGSTATE_SIZE, x31
	csrw		sepc, a0

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
	lw		a2, 0(a0)

	# Figure out which task to send a notification to.

	# Set type argument (0 == external interrupt)
	li		a1, 0
	# Set address, which is -1 for the kernel
	li		a0, -1

	# Restore the stack (x2, sp), global (x3, gp), & thread (x4, tp) pointer
	load_gp_regs 2, 4, x31
	
	# Clear all the other registers (i.e. _not_ x10-x12 / a0-a2)
	clear_gp_regs 1, 1
	clear_gp_regs 5, 9
	clear_gp_regs 13, 31

	# Jump to notification handler
	sret
