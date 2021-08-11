## Interrupt handler designed to work with PLICs

.equ	PLIC_STRIDE_CONTEXT	,	0x1000
.equ	PLIC_CLAIM_OFFSET	,	(0x20 * 0x10000 + 0x4)

external_interrupt_handler:

	# Save some integer registers.
	csrrw	x31, sscratch, x31

	# Save some registers so we can get to work (a0-2 == x10-12)
	save_gp_regs	10, 12, x31

	# Set sepc to that of the notification handler.
	gp_load		a0, 2 * GP_REGBYTES + REGSTATE_SIZE, x31
	csrw		sepc, a0

	# Some real fucking hacky shit to figure out how the fuck these motherfucking interrupts work.

	# Enable SUM
	csrr	a0, sstatus
	li		a1, 1 << 18
	or		a0, a0, a1
	csrw	sstatus, a0

	# Find the offset of the claim register.
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
	lw		a1, 0(a0)

	# Disable SUM
	csrr	a2, sstatus
	li		a0, ~(1 << 18)
	and		a2, a2, a0
	csrw	sstatus, a2

	# Set type argument (0 == external interrupt)
	mv		a0, zero
	
	# Restore some registers
	#load_gp_regs	10, 12, x31
	csrrw	x31, sscratch, x31

	# Jump to notification handler
	# FIXME check for the right TID dumbass.
	sret
