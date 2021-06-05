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
	void* address;
};

/**
 * Returns a pointer to an empty client request entry. Returns NULL if none
 * are available.
 */
struct kernel_client_request_entry *dux_reserve_client_request_entry(void);

/**
 * Returns a pointer to an empty server response entry. Returns NULL if none
 * are available.
 */
struct kernel_server_request_entry *dux_reserve_server_request_entry(void);

/**
 * Reserves a range of memory pages. If the address is NULL, the best fitting address is used and
 * returned. If the range cannot be reserved, NULL is returned.
 */
struct dux_reserve_pages dux_reserve_pages(void *address, size_t count);

/**
 * Unreserves a range of memory pages. Returns a non-zero value if an error occured.
 */
uint8_t dux_unreserve_pages(void *address, size_t count);

#endif
