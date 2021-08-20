## Error handling routines

.section .rodata
early_panic_msg:
	.asciz	"Early panic!"
scause_msg:
	.asciz	"scause  0x"
sepc_msg:
	.asciz	"sepc    0x"
stval_msg:
	.asciz	"stval   0x"
satp_msg:
	.asciz	"satp    0x"
sstatus_msg:
	.asciz	"sstatus 0x"
sp_msg:
	.asciz	"sp      0x"

.section .text.cold

# "Mini panic", which is mainly used for debugging non-Rust panics.
.align	2
mini_panic:

	# Backup ra
	mv		t0, ra

	# Print panic message
	la		s0, early_panic_msg
	call	trap_print_msg
	call	trap_print_lf

	# Print scause
	la		s0, scause_msg
	call	trap_print_msg
	csrr	s0, scause
	call	trap_print_num

	# Print sepc
	la		s0, sepc_msg
	call	trap_print_msg
	csrr	s0, sepc
	call	trap_print_num

	# Print stval
	la		s0, stval_msg
	call	trap_print_msg
	csrr	s0, stval
	call	trap_print_num

	# Print sstatus
	la		s0, sstatus_msg
	call	trap_print_msg
	csrr	s0, sstatus
	call	trap_print_num

	# Print satp
	la		s0, satp_msg
	call	trap_print_msg
	csrr	s0, satp
	call	trap_print_num

	# Print sp
	la		s0, sp_msg
	call	trap_print_msg
	mv		s0, sp
	call	trap_print_num

	# Halt forever
0:
	wfi
	j		0b

	# String message printing routine
	# s0: message
trap_print_msg:
	lb		a0, 0(s0)
0:
	li		a7, 1
	li		a6, 0
	ecall
	addi	s0, s0, 1
	lb		a0, 0(s0)
	bnez	a0, 0b
	ret

	# Hexadecimal number printing routine
	# Always 16 digits.
	# Also prints newline
	# s0: number
trap_print_num:
	li		s1, 60
	li		s2, 10
0:
	srl		a0, s0, s1
	andi	a0, a0, 0xf
	blt		a0, s2, 1f
	addi	a0, a0, 'a' - 10 - '0'
1:
	addi	a0, a0, '0'
	li		a7, 1
	li		a6, 0
	ecall
	addi	s1, s1, -4
	bgez	s1, 0b

	# Newline printing routine
trap_print_lf:
3:
	li		a7, 1
	li		a6, 0
	li		a0, 0xa
	ecall
	ret
