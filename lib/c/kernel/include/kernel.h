#ifndef __KERNEL_H
#define __KERNEL_H

#include <stddef.h>
#include <stdint.h>

#define IO_NONE   (0)
#define IO_READ   (1)
#define IO_WRITE  (2)

#define PROT_READ  (0x1)
#define PROT_WRITE (0x2)
#define PROT_EXEC  (0x4)

/**
 * Structure returned by kernel calls.
 */
typedef struct kernel_return {
	size_t status;
	size_t value;
} kernel_return_t;

/**
 * Structure used by client tasks to send I/O requests.
 */
struct kernel_client_request_entry {
	uint8_t opcode;
	int8_t priority;
	uint16_t flags;
	uint32_t file_handle;
	size_t offset;
	union {
		void *page;
		uint8_t *const filename;
	} data;
	size_t length;
	size_t userdata;
};

/**
 * Structure received by client tasks for I/O completion events.
 */
struct kernel_client_completion_entry {
	union {
		void *page;
		uint32_t file_handle;
	} data;
	size_t length;
	uint32_t status;
	size_t userdata;
};

struct kernel_server_request_entry {
	size_t TODO;
};

struct kernel_server_completion_entry {
	size_t TODO;
};

#define SYSCALL(...) \
	__asm__ __volatile__ ( \
		"ecall\n\t" \
		: "=r"(a0), "=r"(a1) \
		: __VA_ARGS__ \
		: "memory" \
	); \
	kernel_return_t r = { a0, a1 }; \
	return r;

#define SYSCALL_4(__name, __code, __a0, __a1, __a2, __a3) \
	static inline kernel_return_t __name(__a0 a, __a1 b, __a2 c, __a3 d) { \
		register size_t a7 __asm__("a7") = __code; \
		register size_t a0 __asm__("a0") = (size_t)a; \
		register size_t a1 __asm__("a1") = (size_t)b; \
		register size_t a2 __asm__("a2") = (size_t)c; \
		register size_t a3 __asm__("a3") = (size_t)d; \
		SYSCALL("r"(a7), "0"(a0), "1"(a1), "r"(a2), "r"(a3)) \
	}
#define SYSCALL_3(__name, __code, __a0, __a1, __a2) \
	static inline kernel_return_t __name(__a0 a, __a1 b, __a2 c) { \
		register size_t a7 __asm__("a7") = __code; \
		register size_t a0 __asm__("a0") = (size_t)a; \
		register size_t a1 __asm__("a1") = (size_t)b; \
		register size_t a2 __asm__("a2") = (size_t)c; \
		SYSCALL("r"(a7), "0"(a0), "1"(a1), "r"(a2)) \
	}
#define SYSCALL_2(__name, __code, __a0, __a1) \
	static inline kernel_return_t __name(__a0 a, __a1 b) { \
		register size_t a7 __asm__("a7") = __code; \
		register size_t a0 __asm__("a0") = (size_t)a; \
		register size_t a1 __asm__("a1") = (size_t)b; \
		SYSCALL("r"(a7), "0"(a0), "1"(a1)) \
	}
#define SYSCALL_1(__name, __code, __a0) \
	static inline kernel_return_t __name(__a0 a) { \
		register size_t a7 __asm__("a7") = __code; \
		register size_t a0 __asm__("a0") = (size_t)a; \
		register size_t a1 __asm__("a1"); \
		SYSCALL("r"(a7), "0"(a0)) \
	}
#define SYSCALL_0(__name, __code) \
	static kernel_return_t __name() { \
		register size_t a7 __asm__("a7") = __code; \
		register size_t a0 __asm__("a0"); \
		register size_t a1 __asm__("a1"); \
		SYSCALL("r"(a7)) \
	}

SYSCALL_2(kernel_io_wait, 0, uint16_t /* flags */, uint64_t /* time */)

SYSCALL_4(kernel_io_set_client_buffers, 1, void * /* requests */, size_t /* requests_size */,
	void * /* completions */, size_t /* completion_sizes */)

SYSCALL_4(kernel_io_set_server_buffers, 2, void * /* requests */, size_t /* requests_size */,
	void * /* completions */, size_t /* completion_sizes */)

SYSCALL_3(kernel_mem_alloc, 3, void * /* address */, size_t /* count */, uint8_t /* flags */)

SYSCALL_2(kernel_mem_dealloc, 4, void * /* address */, size_t /* count */)

SYSCALL_1(kernel_mem_get_flags, 5, void * /* address */)

SYSCALL_2(kernel_mem_set_flags, 6, void * /* address */, size_t /* count */)

SYSCALL_2(kernel_sys_log, 15, const char * /* address */, size_t /* length */)

#undef SYSCALL_4
#undef SYSCALL_3
#undef SYSCALL_2
#undef SYSCALL_1
#undef SYSCALL_0

/**
 * Convienence macro that expands to kernel_sys_log. Intended for use with literals.
 *
 * If the string is not a literal, call kernel_sys_log directly instead.
 */
#define KERNEL_LOG(msg) kernel_sys_log(msg, sizeof(msg) - 1)

#endif
