# TODO change for RV32
.equ	REGBYTES, 8
.equ	REGORDER, 3

## Save only the caller-saved integer registers. Normally used in the interrupt prologue.
##
## The first argument indicates which register the pointer to the `RegisterState` is located in.
.macro save_caller_registers state_reg
	sd	ra, 0 * REGBYTES (\state_reg)
	sd	t0, 1 * REGBYTES (\state_reg)
	sd	t1, 2 * REGBYTES (\state_reg)
	sd	t2, 3 * REGBYTES (\state_reg)
	sd	t3, 4 * REGBYTES (\state_reg)
	sd	t4, 5 * REGBYTES (\state_reg)
	sd	t5, 6 * REGBYTES (\state_reg)
	sd	t6, 7 * REGBYTES (\state_reg)
	sd	a0, 8 * REGBYTES (\state_reg)
	sd	a1, 9 * REGBYTES (\state_reg)
	sd	a2, 10 * REGBYTES (\state_reg)
	sd	a3, 11 * REGBYTES (\state_reg)
	sd	a4, 12 * REGBYTES (\state_reg)
	sd	a5, 13 * REGBYTES (\state_reg)
	sd	a6, 14 * REGBYTES (\state_reg)
	sd	a7, 15 * REGBYTES (\state_reg)
.endm

## Load only the caller-saved integer registers. Normally used in the interrupt epilogue.
##
## The first argument indicates which register the pointer to the `RegisterState` is located in.
.macro load_caller_registers state_reg
	ld	ra, 0 * REGBYTES (\state_reg)
	ld	t0, 1 * REGBYTES (\state_reg)
	ld	t1, 2 * REGBYTES (\state_reg)
	ld	t2, 3 * REGBYTES (\state_reg)
	ld	t3, 4 * REGBYTES (\state_reg)
	ld	t4, 5 * REGBYTES (\state_reg)
	ld	t5, 6 * REGBYTES (\state_reg)
	ld	t6, 7 * REGBYTES (\state_reg)
	ld	a0, 8 * REGBYTES (\state_reg)
	ld	a1, 9 * REGBYTES (\state_reg)
	ld	a2, 10 * REGBYTES (\state_reg)
	ld	a3, 11 * REGBYTES (\state_reg)
	ld	a4, 12 * REGBYTES (\state_reg)
	ld	a5, 13 * REGBYTES (\state_reg)
	ld	a6, 14 * REGBYTES (\state_reg)
	ld	a7, 15 * REGBYTES (\state_reg)
.endm

## Save only the callee-saved integer registers _except_ `ra`. Normally used for context switching.
.macro save_callee_registers state_reg
	sd	sp, 16 * REGBYTES (\state_reg)
	sd	gp, 17 * REGBYTES (\state_reg)
	sd	tp, 18 * REGBYTES (\state_reg)
	sd	s0, 19 * REGBYTES (\state_reg)
	sd	s1, 20 * REGBYTES (\state_reg)
	sd	s2, 21 * REGBYTES (\state_reg)
	sd	s3, 22 * REGBYTES (\state_reg)
	sd	s4, 23 * REGBYTES (\state_reg)
	sd	s5, 24 * REGBYTES (\state_reg)
	sd	s6, 25 * REGBYTES (\state_reg)
	sd	s7, 26 * REGBYTES (\state_reg)
	sd	s8, 27 * REGBYTES (\state_reg)
	sd	s9, 28 * REGBYTES (\state_reg)
	sd	s10, 29 * REGBYTES (\state_reg)
	sd	s11, 30 * REGBYTES (\state_reg)
.endm

.macro load_callee_registers state_reg
	ld	sp, 16 * REGBYTES (\state_reg)
	ld	gp, 17 * REGBYTES (\state_reg)
	ld	tp, 18 * REGBYTES (\state_reg)
	ld	s0, 19 * REGBYTES (\state_reg)
	ld	s1, 20 * REGBYTES (\state_reg)
	ld	s2, 21 * REGBYTES (\state_reg)
	ld	s3, 22 * REGBYTES (\state_reg)
	ld	s4, 23 * REGBYTES (\state_reg)
	ld	s5, 24 * REGBYTES (\state_reg)
	ld	s6, 25 * REGBYTES (\state_reg)
	ld	s7, 26 * REGBYTES (\state_reg)
	ld	s8, 27 * REGBYTES (\state_reg)
	ld	s9, 28 * REGBYTES (\state_reg)
	ld	s10, 29 * REGBYTES (\state_reg)
	ld	s11, 30 * REGBYTES (\state_reg)
.endm

## Save/load the program counter from/into the given register
.macro save_pc_register state_reg reg
	sd	\reg, 31 * REGBYTES (\state_reg)
.endm

## Save/load the program counter from/into the given register
.macro load_pc_register state_reg reg
	ld	\reg, 31 * REGBYTES (\state_reg)
.endm

## Save/load all the floating point registers. Normally used for context switching.
.macro save_float_registers state_reg
.macro	store	reg, offt
.if 0	# TODO detect features at compile time
	fld	\reg, \offt * REGBYTES (\state_reg)
.elseif 0
	flw	\reg, \offt * REGBYTES (\state_reg)
.endif
.endm
	store	f0, 32
	store	f1, 33
	store	f2, 34
	store	f3, 35
	store	f4, 36
	store	f5, 37
	store	f6, 38
	store	f7, 39
	store	f8, 40
	store	f9, 41
	store	f10, 42
	store	f11, 43
	store	f12, 44
	store	f13, 45
	store	f14, 46
	store	f15, 47
	store	f16, 48
	store	f17, 49
	store	f18, 50
	store	f10, 51
	store	f20, 52
	store	f21, 53
	store	f22, 54
	store	f23, 55
	store	f24, 56
	store	f25, 57
	store	f26, 58
	store	f27, 59
	store	f28, 60
	store	f29, 61
	store	f30, 62
	store	f31, 63
.endm

.macro load_float_registers state_reg
.macro	load	reg, offt
.if 0	# TODO detect features at compile time
	fld	\reg, \offt * REGBYTES (\state_reg)
.elseif 0
	flw	\reg, \offt * REGBYTES (\state_reg)
.endif
.endm
	load	f0, 32
	load	f1, 33
	load	f2, 34
	load	f3, 35
	load	f4, 36
	load	f5, 37
	load	f6, 38
	load	f7, 39
	load	f8, 40
	load	f9, 41
	load	f10, 42
	load	f11, 43
	load	f12, 44
	load	f13, 45
	load	f14, 46
	load	f15, 47
	load	f16, 48
	load	f17, 49
	load	f18, 50
	load	f10, 51
	load	f20, 52
	load	f21, 53
	load	f22, 54
	load	f23, 55
	load	f24, 56
	load	f25, 57
	load	f26, 58
	load	f27, 59
	load	f28, 60
	load	f29, 61
	load	f30, 62
	load	f31, 63
.endm
