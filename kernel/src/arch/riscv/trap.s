## Trap handling routines
##
## These are implemented separately due to naked functions being too inflexible and cumbersome.

#.include	"registers.s"

.equ		USTATUS,		0x000
.equ		UIE,			0x004
.equ		UTVEC,			0x005

.equ		USCRATCH,		0x040
.equ		UEPC,			0x041
.equ		UCAUSE,			0x042
.equ		UTVAL,			0x043
.equ		UIP,			0x044

.equ		FFLAGS,			0x001
.equ		FRM,			0x002
.equ		FCSR,			0x003

.equ		CYCLE,			0xc00
.equ		TIME,			0xc01
.equ		INSTRET,		0xc02

.equ		CYCLEH,			0xc80
.equ		TIMEH,			0xc81
.equ		INSTRETH,		0xc82

.equ		SSTATUS,		0x100
.equ		SEDELEG,		0x102
.equ		SIDELEG,		0x103
.equ		SIE,			0x104
.equ		STVEC,			0x105
.equ		SCOUNTEREN,		0x106

.equ		SSCRATCH,		0x140
.equ		SEPC,			0x141
.equ		SCAUSE,			0x142
.equ		STVAL,			0x143
.equ		SIP,			0x144

.equ		SATP,			0x180

.equ		MVENDORID,		0xf11
.equ		MARCHID,		0xf12
.equ		MIMP,			0xf13
.equ		MHARTID,		0xf14

.equ		MSTATUS,		0x300
.equ		MISA,			0x301
.equ		MEDELEG,		0x302
.equ		MIDELEG,		0x303
.equ		MIE,			0x304
.equ		MTVEC,			0x305
.equ		MCOUNTEREN,		0x306

.equ		MSCRATCH,		0x340
.equ		MEPC,			0x341
.equ		MCAUSE,			0x342
.equ		MTVAL,			0x343
.equ		MIP,			0x344

.equ		MCYCLE,			0xb00
.equ		MINSTRET,		0xb02

.equ		MCYCLEH,		0xb80
.equ		MINSTRETH,		0xb02

.equ		MCOUNTINHIBIT,	0x320

.equ		TSELECT,		0x7a0

.equ		DCSR,			0x7b0
.equ		DPC,			0x7b1

.equ		SYSCALL_MAX,			2
.equ		SYSCALL_ERR_NOCALL, 	1

.globl trap_handler
.globl trap_init

.section .text

## Default handler for traps
trap_handler:
	.align	4
	# Check if it's a syscall. In the case of a syscall, we can skip saving all
	# caller-saved registers.
	csrrw	t0, MCAUSE, t0
	li		t1, 9		# S-mode
	beq		t0, t1, 0f
	li		t1, 11		# M-mode
	beq		t0, t1, 0f
	li		t1, 8		# U-mode
	beq		t0, t1, 0f

	addi	sp, sp, -REGBYTES * 16
	# Restore the register we just overwrote.
	csrrw	t0, MCAUSE, t0
	# Save only registers that are normally caller-saved since we will immediately
	# call after this. The callee will preserve the remaining registers for us.
	save_caller_registers sp

	# TODO implement exception table somewhere
2:
	wfi
	j		2b

	load_caller_registers sp
	addi	sp, sp, REGBYTES * 16
	mret

0:
	addi	sp, sp, -1 * REGBYTES

	# Skip the ecall instruction, which is always 4 bytes long (there is no
	# compressed version of it).
	csrr	t0, MEPC
	addi	t0, t0, 4
	csrw	MEPC, t0

	# Check if the syscall exists, otherwise return the 'no syscall' error code
	li		t1, SYSCALL_MAX
	bgeu	a7, t1, 1f

	# Look up the entry in the call table
	la		t0, syscall_table
	slli	a7, a7, REGORDER
	add		t0, t0, a7
	ld		t0, 0(t0)

	# Perform the call
	sd		ra, 0 * REGBYTES(sp)
	jalr	ra, t0
	ld		ra, 0 * REGBYTES(sp)

3:
	addi	sp, sp, 1 * REGBYTES
	mret

1:
	li		a0, SYSCALL_ERR_NOCALL
	# This is slightly more compact. It is slower, but that's fine for an error path
	j		3b


## Initialize the trap CSR and the interrupt table
trap_init:
	la		t0, trap_handler
	csrw	MTVEC, t0
