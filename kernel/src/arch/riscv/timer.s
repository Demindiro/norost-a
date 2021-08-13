timer_interrupt_handler:

	# Save all the general purpose registers.
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

	beqz	sp, mini_panic

	# Make the executor go to the next call
	j		executor_next_task
