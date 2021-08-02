#include "dux.h"
#include "kernel.h"

#define PAGE_SIZE (0x1000)
#define NULL ((void *)0)

/**
 * The end address of the null page. The null page can never be allocated.
 */
#define NULL_PAGE_END ((void *)0xfff)

/**
 * A single memory range.
 */
struct memory_map {
	void *start;
	// end is *inclusive*, i.e. it can be addressed without a pagefault.
	void *end;
};

/**
 * A sorted list of reserved memory ranges.
 */
static struct memory_map *reserved_ranges;
static size_t reserved_count;
static size_t reserved_capacity;

static struct kernel_ipc_packet *txq;
static size_t txq_mask;
static size_t txq_index;

static struct kernel_ipc_packet *rxq;
static size_t rxq_mask;
static size_t rxq_index;

static struct kernel_free_range *free_ranges;
static size_t free_ranges_size;


/**
 * Initializes the library. This should be the first function called in crt0.
 */
void __dux_init(void)
{
	kernel_return_t kret;
	struct dux_reserve_pages dret;
	// FIXME need a mem_get_mappings syscall of sorts.
	
	// Allocate a single page for keeping track of the memory ranges.
	// FIXME changed top 4 bits from f to 0 to workaround shitty kernel.
	reserved_ranges = (struct memory_map *)0x0ff00000;
	void *reserved_ranges_end = (void *)0x0ff0efff; // 64KiB, or 4096 entries ought to be enough.
	kret = kernel_mem_alloc(reserved_ranges, 1, PROT_READ | PROT_WRITE);
	if (kret.status != 0) {
		// FIXME handle errors properly
		for (;;) {}
	}
	reserved_capacity = PAGE_SIZE / sizeof(struct memory_map);

	// Immediately register the range itself and reserve some pages for it.
	reserved_ranges[/* 0 */1].start = reserved_ranges,
	reserved_ranges[/* 0 */1].end = reserved_ranges_end;
	//reserved_count = 1;

	// FIXME assume the top and bottom are reserved for stack and ELF respectively.
	reserved_ranges[2].start = (void *)0xfff00000,
	reserved_ranges[2].end   = (void *)0xfffeffff,
	reserved_ranges[0].start = (void *)   0x10000,
	reserved_ranges[0].end   = (void *) 0x1ffffff,
	reserved_count = 3;

	// Reserve pages for client requests and responses
	dret = dux_reserve_pages(NULL, 8);
	if (dret.status != 0) {
		// FIXME handle errors properly
		for (;;) {}
	}
	kret = kernel_mem_alloc(dret.address, 1, PROT_READ | PROT_WRITE);
	if (kret.status != 0) {
		// FIXME handle errors properly
		for (;;) {}
	}
	txq = (struct kernel_ipc_packet *)dret.address;
	txq_mask = (PAGE_SIZE / sizeof(struct kernel_ipc_packet)) - 1;
	txq_index = 0;

	dret = dux_reserve_pages(NULL, 8);
	if (dret.status != 0) {
		// FIXME handle errors properly
		for (;;) {}
	}
	kret = kernel_mem_alloc(dret.address, 1, PROT_READ | PROT_WRITE);
	if (kret.status != 0) {
		// FIXME handle errors properly
		for (;;) {}
	}
	rxq = (struct kernel_ipc_packet *)dret.address;
	rxq_mask = (PAGE_SIZE / sizeof(struct kernel_ipc_packet)) - 1;
	rxq_index = 0;

	dret = dux_reserve_pages(NULL, 8);
	if (dret.status != 0) {
		// FIXME handle errors properly
		for (;;) {}
	}
	kret = kernel_mem_alloc(dret.address, 1, PROT_READ | PROT_WRITE);
	if (kret.status != 0) {
		// FIXME handle errors properly
		for (;;) {}
	}
	free_ranges = (struct kernel_free_range *)dret.address;
	free_ranges_size = 1;
	// Set a range to which pages can be mapped to.
	free_ranges[0].address = (void *)0x660000;
	free_ranges[0].count = 1;

	// Register the queues to the kernel
	kret = kernel_io_set_queues(txq, 0, rxq, 0, free_ranges, free_ranges_size);
	if (kret.status != 0) {
		// FIXME handle errors properly
		for (;;) {}
	}
}

/**
 * Insert a memory reservation entry. The index must be lower than reserved_count.
 *
 * Returns 0 on success, 1 if there is not enough space to insert an entry and allocation
 * of extra pages failed.
 */
static uint8_t mem_insert_entry(size_t index, void *start, void *end) {
	// TODO allocate additional pages if needed.
	if (reserved_count >= reserved_capacity) {
		return 1;
	}
	// Shift all entries at and after the index up.
	for (size_t i = reserved_count; i > index; i--) {
		reserved_ranges[i] = reserved_ranges[i - 1];
	}
	reserved_count += 1;
	// Write the entry.
	reserved_ranges[index].start = start;
	reserved_ranges[index].end = end;
	return 0;
}

struct dux_reserve_pages dux_reserve_pages(void *address, size_t count) {
	if (address == NULL) {
		// Find the first range with enough space.
		// TODO maybe it's better if we try to find the tightest space possible? Or maybe
		// the widest space instead?
		void *prev_end = NULL_PAGE_END;
		for (size_t i = 0; i < reserved_count; i++) {
			struct memory_map mm = reserved_ranges[i];
			void *start = prev_end + 1;
			void *end = start + (count * PAGE_SIZE) - 1;
			if (prev_end < start && end < mm.start) {
				// There is enough space, so use it.
				uint8_t r = mem_insert_entry(i, start, end);
				if (r != 0) {
					struct dux_reserve_pages ret = {
						.status = DUX_RESERVE_NOMEM,
						.address = NULL,
					};
					return ret;
				}
				struct dux_reserve_pages ret = {
					.status = DUX_RESERVE_OK,
					.address = start,
				};
				return ret;
			}
			prev_end = mm.end;
		}
		struct dux_reserve_pages ret = {
			.status = DUX_RESERVE_NOSPACE,
			.address = NULL,
		};
		return ret;
	} else {
		// TODO do a binary search, check if there is enough space & insert if so.
		for (;;) {}
	}
}

struct kernel_ipc_packet *dux_reserve_transmit_entry(void) {
	return txq;
}

struct kernel_ipc_packet *dux_get_receive_entry(void) {
	return rxq;
}

int dux_add_free_range(void *page, size_t count) {
	// FIXME
	free_ranges[0].address = page;
	free_ranges[0].count = count;
	return 0;
}
