#include "common.h"
#include "kernel.h"
#include "sys/mman.h"
#include "dux.h"

void *universal_buffer;
size_t universal_buffer_size;

void __posix_init(void)
{
#define NULL ((void *)0)
	asm volatile ("fence");
	struct dux_reserve_pages dret = dux_reserve_pages(NULL, 16);
	asm volatile ("fence");
	if (dret.status != 0) {
		for (;;) {
		}		// TODO
	}
	asm volatile ("fence");
	kernel_return_t kret =
	    kernel_mem_alloc(dret.address, 1, PROT_READ | PROT_WRITE);
	asm volatile ("fence");
	if (kret.status != 0) {
		for (;;) {
		}		// TODO
	}
	asm volatile ("fence");
	universal_buffer = dret.address;
	asm volatile ("fence");
#define PAGE_SIZE (4096)
	universal_buffer_size = 16 * PAGE_SIZE;
	asm volatile ("fence");
}
