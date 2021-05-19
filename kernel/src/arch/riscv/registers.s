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
	sd	reg, 31 * REGBYTES (\state_reg)
}
.endm

## Save/load the program counter from/into the given register
.macro load_pc_register state_reg reg
	ld	reg, 31 * REGBYTES (\state_reg)
}
.endm

## Save/load all the floating point registers. Normally used for context switching.
.macro save_float_registers state_reg
	sd	f0, 32 * REGBYTES (\state_reg)
	sd	f1, 33 * REGBYTES (\state_reg)
	sd	f2, 34 * REGBYTES (\state_reg)
	sd	f3, 35 * REGBYTES (\state_reg)
	sd	f4, 36 * REGBYTES (\state_reg)
	sd	f5, 37 * REGBYTES (\state_reg)
	sd	f6, 38 * REGBYTES (\state_reg)
	sd	f7, 39 * REGBYTES (\state_reg)
	sd	f8, 40 * REGBYTES (\state_reg)
	sd	f9, 41 * REGBYTES (\state_reg)
	sd	f10, 42 * REGBYTES (\state_reg)
	sd	f11, 43 * REGBYTES (\state_reg)
	sd	f12, 44 * REGBYTES (\state_reg)
	sd	f13, 45 * REGBYTES (\state_reg)
	sd	f14, 46 * REGBYTES (\state_reg)
	sd	f15, 47 * REGBYTES (\state_reg)
	sd	f16, 48 * REGBYTES (\state_reg)
	sd	f17, 49 * REGBYTES (\state_reg)
	sd	f18, 50 * REGBYTES (\state_reg)
	sd	f10, 51 * REGBYTES (\state_reg)
	sd	f20, 52 * REGBYTES (\state_reg)
	sd	f21, 53 * REGBYTES (\state_reg)
	sd	f22, 54 * REGBYTES (\state_reg)
	sd	f23, 55 * REGBYTES (\state_reg)
	sd	f24, 56 * REGBYTES (\state_reg)
	sd	f25, 57 * REGBYTES (\state_reg)
	sd	f26, 58 * REGBYTES (\state_reg)
	sd	f27, 59 * REGBYTES (\state_reg)
	sd	f28, 60 * REGBYTES (\state_reg)
	sd	f29, 61 * REGBYTES (\state_reg)
	sd	f30, 62 * REGBYTES (\state_reg)
	sd	f31, 63 * REGBYTES (\state_reg)
.endm

.macro load_float_registers state_reg
	ld	f0, 32 * REGBYTES (\state_reg)
	ld	f1, 33 * REGBYTES (\state_reg)
	ld	f2, 34 * REGBYTES (\state_reg)
	ld	f3, 35 * REGBYTES (\state_reg)
	ld	f4, 36 * REGBYTES (\state_reg)
	ld	f5, 37 * REGBYTES (\state_reg)
	ld	f6, 38 * REGBYTES (\state_reg)
	ld	f7, 39 * REGBYTES (\state_reg)
	ld	f8, 40 * REGBYTES (\state_reg)
	ld	f9, 41 * REGBYTES (\state_reg)
	ld	f10, 42 * REGBYTES (\state_reg)
	ld	f11, 43 * REGBYTES (\state_reg)
	ld	f12, 44 * REGBYTES (\state_reg)
	ld	f13, 45 * REGBYTES (\state_reg)
	ld	f14, 46 * REGBYTES (\state_reg)
	ld	f15, 47 * REGBYTES (\state_reg)
	ld	f16, 48 * REGBYTES (\state_reg)
	ld	f17, 49 * REGBYTES (\state_reg)
	ld	f18, 50 * REGBYTES (\state_reg)
	ld	f10, 51 * REGBYTES (\state_reg)
	ld	f20, 52 * REGBYTES (\state_reg)
	ld	f21, 53 * REGBYTES (\state_reg)
	ld	f22, 54 * REGBYTES (\state_reg)
	ld	f23, 55 * REGBYTES (\state_reg)
	ld	f24, 56 * REGBYTES (\state_reg)
	ld	f25, 57 * REGBYTES (\state_reg)
	ld	f26, 58 * REGBYTES (\state_reg)
	ld	f27, 59 * REGBYTES (\state_reg)
	ld	f28, 60 * REGBYTES (\state_reg)
	ld	f29, 61 * REGBYTES (\state_reg)
	ld	f30, 62 * REGBYTES (\state_reg)
	ld	f31, 63 * REGBYTES (\state_reg)
.endm
