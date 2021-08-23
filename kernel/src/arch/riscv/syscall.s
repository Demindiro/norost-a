# Assembly implementations of some syscalls

# a0: pointer to task struct
syscall_io_notify_return:

	# Indicate the task has finished running its notification handler
	lhu		t0, TASK_FLAGS (a0)
	ori		t0, t0, TASK_FLAG_NOTIFIED

	# Check if an IRQ was triggered
	lw		t1, TASK_IRQ (a0)
	beqz	t1, 0f
 
	# Load PLIC base address
	la      t2, plic_address            # TODO should be a constant, ideally.
	gp_load t2, 0, t2

	# Offset to claim base address
	li      t3, PLIC_CLAIM_OFFSET
	add     t2, t2, t3

	# Offset by context
	li      t3, PLIC_STRIDE_CONTEXT
	li      t4, 1 # FIXME context ID stub
	mul     t3, t3, t4
	add     t2, t2, t3

	# Complete the IRQ
	sw		t1, 0(t2)
	
	# Clear the task's IRQ field
	sw		zero, TASK_IRQ (a0)
	j		1f

0:
	# Clear the notify flag
	andi	t0, t0, ~TASK_FLAG_NOTIFYING

1:
	# Move a0 to x31 as we will overwrite the former
	mv		x31, a0

	# Store the final flags
	# FIXME needs to be atomic
	sh		t0, TASK_FLAGS (x31)

	# Setup the VMS
	ld		t0, TASK_VMS (x31)
	csrw	satp, t0
	sfence.vma

	# Setup sscratch
	csrw	sscratch, x31

	# Restore sp (x2)
	load_gp_regs	2, 2, x31

	# Enable SUM
	li		t0, 1 << 18
	csrs	sstatus, t0

	# Pop a[017] and pc from the stack
	gp_load		a0, -4 * GP_REGBYTES, sp
	gp_load		a1, -3 * GP_REGBYTES, sp
	gp_load		a7, -2 * GP_REGBYTES, sp
	gp_load		t0, -1 * GP_REGBYTES, sp
	csrw		sepc, t0

	# Disable SUM
	csrc	sstatus, t0

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


# a0: pointer to current task struct
# a1: address of the current task
# a2: address of the task we will switch to
syscall_io_notify_defer:

	# Just rerun the handler if userspace is doing something weird / buggy
	# TODO
	bne		a0, a1, 66f
	li		t0, 0x40404040
	csrw	scause, t0
	csrw	stval, a0
	j		mini_panic
66:

	# Get the a[01] from the notification handler that just ran
	# FIXME find a way to properly pass the type between defers
	# the task's a0 is being used to indicate deferment.
	# Perhaps require the task to store the original value on 0(sp)?
	gp_load		s0, 10 * GP_REGBYTES, a0
	gp_load		s1, 11 * GP_REGBYTES, a0

	# Restore sp (x2)
	gp_load		t0, 2 * GP_REGBYTES, a0

	# Enable SUM
	li		t1, 1 << 18
	csrs	sstatus, t1

	# Pop a[017] and pc from the stack
	gp_load		t2, -4 * GP_REGBYTES, t0
	gp_store	t2, 10 * GP_REGBYTES, a0
	gp_load		t2, -3 * GP_REGBYTES, t0 
	gp_store	t2, 11 * GP_REGBYTES, a0
	gp_load		t2, -2 * GP_REGBYTES, t0
	gp_store	t2, 17 * GP_REGBYTES, a0
	gp_load		t2, -1 * GP_REGBYTES, t0
	gp_store	t2,  0 * GP_REGBYTES, a0

	# Disable SUM
	csrc	sstatus, t1

	# Load the IRQ value and clear it.
	lw		s2, TASK_IRQ (a0)
	sw		zero, TASK_IRQ (a0)

	# Indicate the task has finished running its notification handler
	lh		t0, TASK_FLAGS (a0)
	ori		t0, t0, TASK_FLAG_NOTIFIED
	sh		t0, TASK_FLAGS (a0)

	# Save address for notification_enter
	mv		s3, a1

	# Get a pointer to the task with the given address
	mv		a0, a2
	call	executor_get_task
	# FIXME don't panic you doof
	beqz	a0, mini_panic

	# Check if an IRQ was triggered and handle appropriately
	beqz	s2, 0f

	# Set the IRQ in the new task's field whenever said task is done
	addi	t1, a0, TASK_IRQ
3:
	# LR is mandatory, bah -_-
	lr.w	t2, (t1)
	bnez	t2, 3b
	sc.w	t2, s2, (t1)
	beqz	t2, 1f
	j		3b

0:
	# Set the notify flag
	# FIXME needs to be atomic
	lhu		a3, TASK_FLAGS (a0)
	ori		a3, a3, TASK_FLAG_NOTIFYING
	sh		a3, TASK_FLAGS (a0)

1:

	# Enter notification handler
	mv		x31, a0
	mv		a0, s0
	mv		a1, s1
	mv		a7, s3
	j		notification_enter
