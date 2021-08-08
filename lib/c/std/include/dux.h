#ifndef __DUX_H
#define __DUX_H

#include "kernel.h"
#include "stdint.h"

#define DUX_RESERVE_OK      (0)
#define DUX_RESERVE_NOSPACE (1)
#define DUX_RESERVE_NOMEM   (2)

/**
 * Structure returned by dux_reserve_pages. A negative status field indicates an error.
 *
 * Possible error values:
 *
 * -1 means no more ranges were free.
 *
 * -2 means the address wasn't properly aligned.
 *
 * -3 means there was no more physical memory available to add the entry.
 */
struct dux_reserve_pages {
	void *address;
	int8_t status;
};

/**
 * A single, raw child object entry.
 */
struct dux_ipc_list_raw_entry {
	kernel_uuid_t uuid;
	uint64_t size;
	uint32_t name_offset;
	uint16_t name_len;
};

/**
 * A child object entry returned by dux_ipc_list_get
 */
struct dux_ipc_list_entry {
	kernel_uuid_t uuid;
	uint64_t size;
	const char *name;
	uint16_t name_len;
};

/**
 * A list of child objects.
 */
struct dux_ipc_list {
	void *data;
	size_t data_len;
};

/**
 * Returns a slot with an empty client request entry. Returns -1 if none
 * are available.
 *
 * This locks the transmit ring buffer until a packet is submitted.
 *
 * The `packet` parameter **MUST** not be NULL!
 */
uint16_t dux_reserve_transmit_entry(struct kernel_ipc_packet **packet);

/**
 * Submits a packet previously received with `dux_reserve_transmit_entry`. This
 * unlocks the transmit ring buffer.
 */
void dux_submit_transmit_entry(uint16_t slot);

/**
 * Return a pointer to the current receive entry. Returns -1 if no unprocessed packets
 * are available.
 */
uint16_t dux_get_received_entry(const struct kernel_ipc_packet **packet);

/**
 * "Pop" the received entry from the received list, readding it to the free stack
 *
 * This **MUST** be called only once per slot!
 */
void dux_pop_received_entry(uint16_t slot);

/**
 * Readd the slot to the received list. This is useful if a function is
 * waiting for a specific packet.
 *
 * This **MUST** be called only once per slot!
 */
void dux_defer_received_entry(uint16_t slot);

/**
 * Reserves a range of memory pages. If the address is NULL, the best fitting address is used and
 * returned. If the range cannot be reserved, NULL is returned.
 */
struct dux_reserve_pages dux_reserve_pages(void *address, size_t count);

/**
 * Unreserves a range of memory pages. Returns a negative value if an error occured.
 *
 * # Returns
 *
 * 0 on success.
 *
 * -1 if the address is either NULL or not properly aligned.
 *
 * -2 if the range isn't reserved.
 *
 * -3 if a matching entry was found but the count is too large.
 */
int dux_unreserve_pages(void *address, size_t count);

/**
 * Add an address range that can be freely used for IPC
 */
int dux_add_free_range(void *address, size_t count);

/**
 * Return the entry at the given index in this list.
 *
 * Returns -1 if the index is out of range, otherwise 0.
 */
int dux_ipc_list_get(const struct dux_ipc_list *list, size_t index,
		     struct dux_ipc_list_entry *entry);

#endif
