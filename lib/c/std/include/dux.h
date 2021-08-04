#ifndef __DUX_H
#define __DUX_H

#include "kernel.h"
#include "stdint.h"

#define DUX_RESERVE_OK      (0)
#define DUX_RESERVE_NOSPACE (1)
#define DUX_RESERVE_NOMEM   (2)

/**
 * Structure returned by dux_reserve_pages. A non-zero status field indicates an error.
 */
struct dux_reserve_pages {
	uint8_t status;
	void *address;
};

/**
 * A single, raw child object entry.
 */
struct dux_ipc_list_raw_entry {
	kernel_uuid_t uuid;
	uint32_t name_offset;
	uint16_t name_len;
};

/**
 * A child object entry returned by dux_ipc_list_get
 */
struct dux_ipc_list_entry {
	kernel_uuid_t uuid;
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
 * Returns a pointer to an empty client request entry. Returns NULL if none
 * are available.
 */
struct kernel_ipc_packet *dux_reserve_transmit_entry(void);

/**
 * Return a pointer to the current receive entry. Returns NULL if there is no receive queue.
 */
struct kernel_ipc_packet *dux_get_receive_entry(void);

/**
 * Reserves a range of memory pages. If the address is NULL, the best fitting address is used and
 * returned. If the range cannot be reserved, NULL is returned.
 */
struct dux_reserve_pages dux_reserve_pages(void *address, size_t count);

/**
 * Unreserves a range of memory pages. Returns a non-zero value if an error occured.
 */
uint8_t dux_unreserve_pages(void *address, size_t count);

/**
 * Add an address range that can be freely used for IPC
 */
int dux_add_free_range(void *address, size_t count);

/**
 * Return the entry at the given index in this list.
 *
 * Returns -1 if the index is out of range, otherwise 0.
 */
int dux_ipc_list_get(const struct dux_ipc_list list, size_t index,
		     struct dux_ipc_list_entry *entry);

#endif
