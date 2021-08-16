.globl _start

.section .data
.align 12
kernel:
	.incbin	"../target/riscv64gc-unknown-none-elf/release/kernel"
kernel_end:
.equ KERNEL_SIZE, . - kernel

.section .data
.align 12
init:
	.incbin "../init.elf"
.equ INIT_SIZE, . - init
init_size:
	.quad	INIT_SIZE

# Macros to ensure we don't accidently invoke UB.
# It won't eliminate all cases but it helps catch common bugs such
# as using a kernel that isn't ELF.

.macro assert_eq, lhs, rhs, msg
	beq		\lhs, \rhs, 9f
	la		a0, \msg
	j		panic
9:
.endm

.macro assert_ne, lhs, rhs, msg
	bne		\lhs, \rhs, 9f
	la		a0, \msg
	j		panic
9:
.endm

.macro assert_ge, lhs, rhs, msg
	bge		\lhs, \rhs, 9f
	la		a0, \msg
	j		panic
9:
.endm

.macro assert_le, lhs, rhs, msg
	assert_ge	\rhs, \lhs, \msg
.endm

# FIXME yikes
.equ	PAGE_ALLOC_BASE_ADDR, 0x83500000

.section .text
_start:

	## Global registers used & meanings:
	#
	#   a0: Hart ID
	#   a1: FDT
	#   a2: kernel start
	#   a3: kernel size
	#   a4: init start
	#   a5: init size
	#
	#   s0: PPN[0]
	#   s1: PPN[1]
	#   s2: PPN[2]
	#
	#   s3: Pointer to start of kernel image
	#   s4: Pointer to end of kernel image
	#   s5: Pointer to start of program headers.
	#   s6: Pointer to end of program headers
	#
	#   s7: Page size (4096, 0x1000)
	#   s8: Page mask for lower bits (0xfff)
	#   s9: Page mask for higher bits (~0xfff)
	#   s10: ELF LOAD constant (1)
	#
	#   s11: page alloc

	# Set up global registers
	li		s11, PAGE_ALLOC_BASE_ADDR

	li		t0, 4096
	mv		s0, s11
	add		s11, s11, t0
	mv		s1, s11
	add		s11, s11, t0
	mv		s2, s11
	add		s11, s11, t0

	la		s3, kernel
	la		s4, kernel_end

	ld		s5, 32(s3)
	add		s5, s3, s5

	lh		s6, 56(s3)
	li		t0, 56
	mul		s6, s6, t0
	add		s6, s5, s6

	li		s7, 4096
	addi	s8, s7, -1
	xori	s9, s8, -1
	li		s10, 1


	## Sanity-check the ELF & insert entries
	li		t0, 0x464c457f		# Check the magic (first 4 bytes ought to be
	lw		t1, 0(s3)			# good enough)
	assert_eq	t0, t1, err_bad_magic
	assert_ne	s5, s6, err_no_program_headers
	assert_le	s6, s4, err_program_headers_out_of_bounds


	## Iterate program headers & insert PTEs
	#
	# Note that s5 will be equivalent to s6 after this.
0:
	lw		t0, 0(s5)			# Check if the segment is of type LOAD,
	bne		t0, s10, 1f			# otherwise skip
	
	ld		t0,  4(s5)			# Get flags
	ld		t1,  8(s5)			# Get offset
	ld		t2, 16(s5)			# Get virtual address
	ld		t6, 32(s5)			# Get file size
	ld		t3, 40(s5)			# Get memory size
	and		t4, t1, s8			# Lower 12 bits of offset.
	and		t1, t1, s9			# Higher 52 bits of PPN

	# Ensure that filesz <= memsz
	assert_ge	t3, t6, err_filesz_ge_memsz

	# Round file size up to a page boundary with (x + mask_offset + PAGE_MASK) & !PAGE_MASK
	# The base address is automatically rounded down by (x >> MASK_BITS)
	add		t6, t6, t4
	add		t6, t6, s8
	and		t6, t6, s9

	# Round memory size up to a page boundary with (x + mask_offset + PAGE_MASK) & !PAGE_MASK
	# The base address is automatically rounded down by (x >> MASK_BITS)
	add		t3, t3, t4
	add		t3, t3, s8
	and		t3, t3, s9

	# Set the actual amount of pages to allocate
	sub		t3, t3, t6

	# Create a PTE for each file page.
	add		t1, s3, t1			# Pointer to physical ELF page
	add		t6, t1, t6			# Pointer to physical end address

	srli	t2, t2, 12			# Get index (pointer) in page table
	andi	t2, t2, 0x1ff		# TODO check if upper bits are all on
	slli	t2, t2, 3

	j		3f
2:
	# Create PTE
	srli	t4, t1, 12			# Set PPN[2:0]
	slli	t4, t4, 10
	ori		t4, t4, 0xc1		# Set valid, dirty & accessed bits
	ori		t4, t4, 0x0e		# TODO set RWX flags properly

	# Store PTE
	li			t5, 511 * 8
	assert_le	t2, t5, err_pte_oob
	add			t5, s0, t2
	sd			t4, 0(t5)

	# Increment & check if another PTE needs to be inserted.
	add		t1, t1, s7
	addi	t2, t2, 8
3:
	bne		t1, t6, 2b

	# TODO should we clear the remaining bytes?

	# Allocate pages for the remaining memory
	add		t3, s11, t3			# Pointer to physical end address

	j		3f
2:
	# Clear memory
	mv		t4, s11
	add		t5, s11, s7
4:
	sd		zero, 0(t4)
	sd		zero, 8(t4)
	sd		zero, 16(t4)
	sd		zero, 24(t4)
	addi	t4, t4, 32
	bne		t4, t5, 4b

	# Create PTE
	srli	t4, s11, 12			# Set PPN[2:0]
	slli	t4, t4, 10
	ori		t4, t4, 0xc1		# Set valid, dirty & accessed bits
	ori		t4, t4, 0x0e		# TODO set RWX flags properly

	# Store PTE
	li			t5, 511 * 8
	assert_le	t2, t5, err_pte_oob
	add			t5, s0, t2
	sd			t4, 0(t5)

	# Increment & check if another PTE needs to be inserted.
	add		s11, s11, s7
	addi	t2, t2, 8
3:
	bne		s11, t3, 2b

1:
	# Perform next iteration or finish.
	addi	s5, s5, 56
	bne		s5, s6, 0b


	# Add pointer to PPN1 in PPN2 & PPN0 in PPN1.
	li		t3, (511 * 8)		# Offset pointing to PTE
	add		t2, s2, t3			# Add offset to point to last PTE
	add		t1, s1, t3
	ld		t4, 0(t2)			# Ensure PTEs are zero
	ld		t3, 0(t1)
	assert_eq	t4, zero, err_pte_pointer_nonzero
	assert_eq	t3, zero, err_pte_pointer_nonzero
	srli	t4, s1, 2			# Set PPN
	srli	t3, s0, 2
	ori		t4, t4, 1			# Set valid bit
	ori		t3, t3, 1
	sd		t4, 0(t2)			# Set PTEs
	sd		t3, 0(t1)

	## Identity map the lower half of memory.
	mv		t2, s2				# Pointer to index 0
	addi	t3, s2, 255 * 8		# Pointer to index 255 (inclusive)
	li		t0, 0xcf			# Start PTE & Set valid, RWX and
								# dirty/accessed bits
	slli	t1, s7, 28 - 12		# PTE/PPN increment (s7 is PAGE_SIZE and
								# li would take more instructions)
0:
	ld		t4, 0(t2)
	assert_eq	t4, zero, err_identity_map_pte_nonzero
	sd		t0, 0(t2)

	# Perform next iteration
	addi	t2, t2, 8
	add		t0, t0, t1
	bne		t2, t3, 0b


	# Now that the page table is properly set up, we can set satp.
	srli	s2, s2, 12			# Set PPN
	li		t0, 1 << 63			# Use Sv39
	or		s2, s2, t0
	csrw	satp, s2

	# Set arguments
	mv		a2, s3
	sub		a3, s4, s3
	la		a4, init
	#li		a5, INIT_SIZE
	la		a5, init_size
	ld		a5, 0(a5)

	# Get the entry point and jump to it.
	ld		ra, 24(s3)
	jalr	zero, ra


panic:
	mv		s0, a0

	# Print "PANIC! "
	la		s1, panic_msg
0:
	li		a7, 0x01
	li		a6, 0
	lb		a0, 0(s1)
	beq		a0, zero, 1f
	ecall
	addi	s1, s1, 1
	j		0b

	# Print message
1:
	li		a7, 0x01
	li		a6, 0
	lb		a0, 0(s0)
	beq		a0, zero, 2f
	ecall
	addi	s0, s0, 1
	j		1b

	# Print newline
2:
	li		a0, 0xa
	ecall

	# Halt
3:
	wfi
	j		3b


panic_msg:
	.asciz	"PANIC! "

err_bad_magic:
	.asciz	"Bad magic"

err_no_program_headers:
	.asciz	"Program headers count is 0"

err_program_headers_out_of_bounds:
	.asciz	"Program headers exceed ELF size"

err_pte_pointer_nonzero:
	.asciz	"PTE pointer is non-zero"

err_identity_map_pte_nonzero:
	.asciz	"Identity map PTE is non-zero"

err_filesz_ge_memsz:
	.asciz	"File size larger than memory size"

err_pte_oob:
	.asciz	"PTE index out of bounds"
