/* Dux-specific headers for extensions to PDCLIB */

#ifndef _PDCLIB_DUX_H
#define _PDCLIB_DUX_H _PDCLIB_DUX_H

/* This macro is used to indicate functions that need to be implemented using */
/* Dux-specific functionality                                                 */
#define DUX_TODO(ret) do { \
		/* TODO print a message */ \
		return ret; \
	} while (0);

#endif
