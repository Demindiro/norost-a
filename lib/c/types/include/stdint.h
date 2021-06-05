#ifndef __STDINT_H
#define __STDINT_H


#ifdef __riscv
typedef signed   long      ssize_t;
typedef signed   long long int64_t;
typedef signed   int       int32_t;
typedef signed   short     int16_t;
typedef signed   char      int8_t;
typedef unsigned long      size_t;
typedef unsigned long long uint64_t;
typedef unsigned int       uint32_t;
typedef unsigned short     uint16_t;
typedef unsigned char      uint8_t;
#else
# error "Unable to detect architecture"
#endif


#endif
