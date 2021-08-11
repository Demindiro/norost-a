## Macros & constants pertaining to saving & restoring registers

# Save the given range of general purpose registers
.macro save_gp_regs		from, to, in
	.altmacro
	# Dumb hack to bypass recursion limit because idk how to pass args to
	# assembler in Rust (this has O(log2(n)) depth instead of O(n))
	.if		\to - \from > 0
		save_gp_regs	\from, %(\from + (\to - \from) / 2), \in
		save_gp_regs	%(\from + (\to - \from) / 2 + 1), \to, \in
	.elseif		\to - \from == 0
		gp_store		x\from, \from * GP_REGBYTES, \in
	.endif
.endm

# Load the given range of general purpose registers
.macro load_gp_regs		from, to, in
	.altmacro
	# Ditto
	.if		\to - \from > 0
		load_gp_regs	\from, %(\from + (\to - \from) / 2), \in
		load_gp_regs	%(\from + (\to - \from) / 2 + 1), \to, \in
	.elseif		\to - \from == 0
		gp_load			x\from, \from * GP_REGBYTES, \in
	.endif
.endm

# Save all floating point registers
.macro save_fp_regs storage
	__index = 0
	.rept	FP_REGCOUNT
		fp_store	f%(__index), GP_REGSTATE_SIZE + %(__index) * FP_REGBYTES (\storage)
		__index = __index + 1
	.endr
.endm


# Load all floating point registers
.macro load_fp_regs storage
	.if		FP_REGCOUNT > 0
		__index = 0
		.rept	FP_REGCOUNT
			fp_load		f\__index, GP_REGSTATE_SIZE + $__index * FP_REGBYTES (\storage)
			__index = __index + 1
		.endr
	.endif
.endm
