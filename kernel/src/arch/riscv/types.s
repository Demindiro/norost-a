## Layout of common structs

# Definitions for general purpose registers
.ifdef	__RISCV64__

	.equ	GP_REGBYTES, 8
	.equ	GP_REGORDER, 3

	.macro gp_load	a, b, c
		ld	\a, \b (\c)
	.endm

	.macro gp_store	a, b, c
		sd	\a, \b (\c)
	.endm

.else
.ifdef	__RISCV32__

	.equ	GP_REGBYTES, 4
	.equ	GP_REGORDER, 2

	.macro gp_load	a, b, c
		lw	\a, \b (\c)
	.endm

	.macro gp_store	a, b, c
		sw	\a, \b (\c)
	.endm

.else

	.error	"Neither __RISCV64__ nor __RISCV32__ was defined"

.endif
.endif


# Size of the general purpose register storage
.ifdef __RV32E__
	.equ	GP_REGSTATE_SIZE, 16 * GP_REGBYTES
.else
	.equ	GP_REGSTATE_SIZE, 32 * GP_REGBYTES
.endif


# Definitions for floating point registers
.ifdef __EXT_D__

	.equ	FP_REGBYTES, 8
	.equ	FP_REGCOUNT, 32

	.macro fp_load	a, b
		ld	\a, \b
	.endm

	.macro fp_store	a, b
		sd	\a, \b
	.endm

.else
.ifdef __EXT_F__

	.equ	FP_REGBYTES, 4
	.equ	FP_REGCOUNT, 32

	.macro fp_load	a, b
		lw	\a, \b
	.endm

	.macro fp_store	a, b
		sw	\a, \b
	.endm

.else

	.equ	FP_REGBYTES, 0
	.equ	FP_REGCOUNT, 0

.endif
.endif


# Size of the floating point register storage
.equ	FP_REGSTATE_SIZE, FP_REGCOUNT * FP_REGBYTES


# Total size of register storage
.equ		REGSTATE_SIZE, (GP_REGSTATE_SIZE + FP_REGSTATE_SIZE)


# The offset of the tasks' stack, which shouldn't even be present in the
# task struct.
.equ		TASK_STACK, REGSTATE_SIZE

# The offset of the task's VMS address
.equ		TASK_VMS, (TASK_STACK + GP_REGBYTES)

# The offset of the task's notification handler
.equ		TASK_NOTIFY_HANDLER, (TASK_VMS + GP_REGBYTES)

# The offset of the task's flags
.equ		TASK_FLAGS, (TASK_NOTIFY_HANDLER + GP_REGBYTES)

# The offset of the task's current IRQ value
.equ		TASK_IRQ, (TASK_FLAGS + 2)

# Mask to toggle the NOTIFY flag
.equ		TASK_FLAG_NOTIFY, 0x1

# The total amount of system calls, including placeholders
.equ		SYSCALL_MAX,			20

# The error code for when a syscall was not found.
.equ		SYSCALL_ERR_NOCALL, 	1
