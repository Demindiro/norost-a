#include "common.h"
#include "kernel.h"
#include "sys/mman.h"


#define REQUEST_QUEUE_ADDRESS    ((void *)0x1000000)
#define COMPLETION_QUEUE_ADDRESS ((void *)0x1001000)
#define UNIVERSAL_BUFFER_ADDRESS ((void *)0x1002000)
#define REQUEST_QUEUE_SIZE    (  64)
#define COMPLETION_QUEUE_SIZE ( 128)
#define UNIVERSAL_BUFFER_SIZE (4096)


struct kernel_client_request_entry *request_queue;
size_t request_mask;
size_t request_index;

struct kernel_client_completion_entry *completion_queue;
size_t completion_mask;
size_t completion_index;

void *universal_buffer;
size_t universal_buffer_size;


void __posix_init(void) {
	kernel_return_t ret;
	
	// FIXME handle errors properly.
	ret = mem_alloc(REQUEST_QUEUE_ADDRESS, 1, PROT_READ | PROT_WRITE);
	if (ret.status != 0) {
		for (;;) {} // TODO
	}
	ret = mem_alloc(COMPLETION_QUEUE_ADDRESS, 1, PROT_READ | PROT_WRITE);
	if (ret.status != 0) {
		for (;;) {} // TODO
	}
	ret = io_set_client_buffers(REQUEST_QUEUE_ADDRESS, 0, COMPLETION_QUEUE_ADDRESS, 0);
	if (ret.status != 0) {
		for (;;) {} // TODO
	}
	ret = mem_alloc(UNIVERSAL_BUFFER_ADDRESS, 1, PROT_READ | PROT_WRITE);
	if (ret.status != 0) {
		for (;;) {} // TODO
	}

	request_queue = REQUEST_QUEUE_ADDRESS;
	completion_queue = COMPLETION_QUEUE_ADDRESS;

	request_mask = REQUEST_QUEUE_SIZE - 1;
	completion_mask = COMPLETION_QUEUE_SIZE - 1;

	universal_buffer = UNIVERSAL_BUFFER_ADDRESS;
	universal_buffer_size = UNIVERSAL_BUFFER_SIZE;
}
