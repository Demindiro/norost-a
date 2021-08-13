#ifndef __KERNEL_H
#define __KERNEL_H

#include <stddef.h>
#include <stdint.h>

#define PROT_READ  (0x1)
#define PROT_WRITE (0x2)
#define PROT_EXEC  (0x4)

#define PAGE_SIZE (0x1000)

/**
 * The type of a task / process identifier
 */
typedef size_t pid_t;

/**
 * An UUID, which is always 128 bits large.
 */
typedef struct {
	uint64_t _x;
	uint64_t _y;
} kernel_uuid_t;

#define KERNEL_UUID(x, y) { ._x = x, ._y = y }

static inline kernel_uuid_t kernel_uuid(uint64_t x, uint64_t y)
{
	kernel_uuid_t uuid = KERNEL_UUID(x, y);
	return uuid;
}

/**
 * Structure returned by kernel calls.
 */
typedef struct kernel_return {
	size_t status;
	size_t value;
} kernel_return_t;

/**
 * Structure used for ipc.
 */
struct kernel_ipc_packet {
	kernel_uuid_t uuid;
	union {
		void *raw;
	} data;
	void *name;
	int64_t offset;
	size_t length;
	pid_t address;
	uint16_t flags;
	uint16_t name_len;
	uint8_t id;
	uint8_t opcode;
};

/**
 * Valid IPC operations
 */
enum {
	KERNEL_IPC_OP_NONE = 0,
	KERNEL_IPC_OP_READ = 1,
	KERNEL_IPC_OP_WRITE = 2,
	KERNEL_IPC_OP_INFO = 3,
	KERNEL_IPC_OP_LIST = 4,
	KERNEL_IPC_OP_MAP_READ = 5,
	KERNEL_IPC_OP_MAP_WRITE = 6,
	KERNEL_IPC_OP_MAP_READ_WRITE = 7,
	KERNEL_IPC_OP_MAP_EXEC = 8,
	KERNEL_IPC_OP_MAP_READ_EXEC = 9,
	KERNEL_IPC_OP_MAP_READ_COW = 10,
	KERNEL_IPC_OP_MAP_EXEC_COW = 11,
	KERNEL_IPC_OP_MAP_READ_EXEC_COW = 12,
};

/**
 * Structure used to indicate IPC ranges where pages can be mapped into.
 */
struct kernel_free_range {
	void *address;
	size_t count;
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

#define SYSCALL_SAVEALL(...) \
	__asm__ __volatile__ ( \
		"ecall\n\t" \
		: \
		: __VA_ARGS__ \
		: "memory" \
	);

#define SYSCALL_6(__name, __code, __a0, __a1, __a2, __a3, __a4, __a5) \
	static inline kernel_return_t __name(__a0 a, __a1 b, __a2 c, __a3 d, __a4 e, __a5 f) { \
		register size_t a7 __asm__("a7") = __code; \
		register size_t a0 __asm__("a0") = (size_t)a; \
		register size_t a1 __asm__("a1") = (size_t)b; \
		register size_t a2 __asm__("a2") = (size_t)c; \
		register size_t a3 __asm__("a3") = (size_t)d; \
		register size_t a4 __asm__("a4") = (size_t)e; \
		register size_t a5 __asm__("a5") = (size_t)f; \
		SYSCALL("r"(a7), "0"(a0), "1"(a1), "r"(a2), "r"(a3), "r"(a4), "r"(a5)) \
	}
#define SYSCALL_5(__name, __code, __a0, __a1, __a2, __a3, __a4) \
	static inline kernel_return_t __name(__a0 a, __a1 b, __a2 c, __a3 d, __a4 e) { \
		register size_t a7 __asm__("a7") = __code; \
		register size_t a0 __asm__("a0") = (size_t)a; \
		register size_t a1 __asm__("a1") = (size_t)b; \
		register size_t a2 __asm__("a2") = (size_t)c; \
		register size_t a3 __asm__("a3") = (size_t)d; \
		register size_t a4 __asm__("a4") = (size_t)e; \
		SYSCALL("r"(a7), "0"(a0), "1"(a1), "r"(a2), "r"(a3), "r"(a4)) \
	}
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
#define SYSCALL_1_SAVEALL(__name, __code, __a0) \
	static inline void __name(__a0 a) { \
		register size_t a7 __asm__("a7") = __code; \
		register size_t a0 __asm__("a0") = (size_t)a; \
		SYSCALL_SAVEALL("r"(a7), "r"(a0)) \
	}

SYSCALL_1_SAVEALL(kernel_io_wait, 0, uint64_t /* time */ )

SYSCALL_6(kernel_io_set_queues, 1, void * /* requests */ ,
	      size_t /* requests_size */ ,
	      void * /* completions */ , size_t /* completion_sizes */ ,
	      void * /* free_pages */ , size_t /* free_pages_size */ )

SYSCALL_3(kernel_mem_alloc, 3, void * /* address */ , size_t /* count */ ,
	  uint8_t /* flags */ ) SYSCALL_2(kernel_mem_dealloc, 4,
					  void * /* address */ ,
					  size_t /* count */ )
SYSCALL_1(kernel_mem_get_flags, 5,
	  void * /* address */ ) SYSCALL_2(kernel_mem_set_flags, 6,
					   void * /* address */ ,
					   size_t /* count */ )
SYSCALL_2(kernel_sys_log, 15, const char * /* address */ , size_t /* length */ )
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
#define KERNEL_LOG(msg) kernel_sys_log(msg "\n", sizeof(msg))
#endif
