# Assembly implementations of some syscalls

# a0: pointer to task struct
syscall_io_notify_return:

	# Move a0 to x31 as we will overwrite the former
	mv		x31, a0

	# Check if an IRQ was triggered
	lh		a3, TASK_IRQ (x31)
	beqz	a3, 0f

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

	# Complete the IRQ
	sw		a3, 0(a0)
	
	# Clear the task's IRQ field
	sh		zero, TASK_IRQ (x31)
	j		1f

0:
	# Clear the notify flag
	# FIXME needs to be atomic
	lh		a3, TASK_FLAGS (x31)
	andi	a3, a3, ~TASK_FLAG_NOTIFY
	sh		a3, TASK_FLAGS (x31)

1:

	# Setup the VMS
	ld		a0, TASK_VMS (x31)
	csrw	satp, a0
	sfence.vma

	# Setup sscratch
	csrw	sscratch, x31

	# Restore sp (x2)
	load_gp_regs	2, 2, x31

	# Enable SUM
	li		a2, 1 << 18
	csrs	sstatus, a2

	# Pop a[017] and pc from the stack
	gp_load		a0, -4 * GP_REGBYTES, sp
	gp_load		a1, -3 * GP_REGBYTES, sp
	gp_load		a7, -2 * GP_REGBYTES, sp
	gp_load		x30, -1 * GP_REGBYTES, sp
	csrw		sepc, x30

	# Disable SUM
	csrc	sstatus, a2

	# == FIXME save the FP registers
	li		t0, 1 << 13
	csrc	sstatus, t0
	li		t0, 1 << 14
	csrs	sstatus, t0
	# ==

	# Restore all registers except a[017] and sp
	load_gp_regs	1, 9, x31
	load_gp_regs	12, 16, x31
	load_gp_regs	18, 31, x31

	# Begin running the task
	sret
