#ifndef __ERRNO_H
#define __ERRNO_H

typedef int errno_t;

extern errno_t errno;

enum {
	/* Result too large */
	ERANGE,         
	/* Mathematics argument out of domain of function */
	EDOM,           
	/* Illegal byte sequence */
	EILSEQ,         
	/* Argument list too long */
	E2BIG,          
	/* Permission denied */
	EACCES,         
	/* Address in use */
	EADDRINUSE,     
	/* Address not available */
	EADDRNOTAVAIL,  
	/* Address family not supported */
	EAFNOSUPPORT,   
	/* Resource unavailable, try again */
	EAGAIN,         
	/* Connection already in progress */
	EALREADY,       
	/* Bad file descriptor */
	EBADF,          
	/* Bad message */
	EBADMSG,        
	/* Device or resource busy */
	EBUSY,          
	/* Operation canceled */
	ECANCELED,      
	/* No child processes */
	ECHILD,         
	/* Connection aborted */
	ECONNABORTED,   
	/* Connection refused */
	ECONNREFUSED,   
	/* Connection reset */
	ECONNRESET,     
	/* Resource deadlock would occur */
	EDEADLK,        
	/* Destination address required */
	EDESTADDRREQ,   
	/* File exists */
	EEXIST,         
	/* Bad address */
	EFAULT,         
	/* File too large */
	EFBIG,          
	/* Host is unreachable */
	EHOSTUNREACH,   
	/* Identifier removed */
	EIDRM,          
	/* Operation in progress */
	EINPROGRESS,    
	/* Interrupted function */
	EINTR,          
	/* Invalid argument */
	EINVAL,         
	/* I/O error */
	EIO,            
	/* Socket is connected */
	EISCONN,        
	/* Is a directory */
	EISDIR,         
	/* Too many levels of symbolic links */
	ELOOP,          
	/* File descriptor value too large */
	EMFILE,         
	/* Too many links */
	EMLINK,         
	/* Message too large */
	EMSGSIZE,       
	/* Filename too long */
	ENAMETOOLONG,   
	/* Network is down */
	ENETDOWN,       
	/* Connection aborted by network */
	ENETRESET,      
	/* Network unreachable */
	ENETUNREACH,    
	/* Too many files open in system */
	ENFILE,         
	/* No buffer space available */
	ENOBUFS,        
	/* No message is available on the STREAM head read queue */
	ENODATA,        
	/* No such device */
	ENODEV,         
	/* No such file or directory */
	ENOENT,         
	/* Executable file format error */
	ENOEXEC,        
	/* No locks available */
	ENOLCK,         
	/* Link has been severed */
	ENOLINK,        
	/* Not enough space */
	ENOMEM,         
	/* No message of the desired type */
	ENOMSG,         
	/* Protocol not available */
	ENOPROTOOPT,    
	/* No space left on device */
	ENOSPC,         
	/* No STREAM resources */
	ENOSR,          
	/* Not a STREAM */
	ENOSTR,         
	/* Function not supported */
	ENOSYS,         
	/* The socket is not connected */
	ENOTCONN,       
	/* Not a directory */
	ENOTDIR,        
	/* Directory not empty */
	ENOTEMPTY,      
	/* State not recoverable */
	ENOTRECOVERABLE,
	/* Not a socket */
	ENOTSOCK,       
	/* Not supported */
	ENOTSUP,        
	/* Inappropriate I/O control operation */
	ENOTTY,         
	/* No such device or address */
	ENXIO,          
	/* Operation not supported on socket */
	EOPNOTSUPP,     
	/* Value too large to be stored in data type */
	EOVERFLOW,      
	/* Previous owner died */
	EOWNERDEAD,     
	/* Operation not permitted */
	EPERM,          
	/* Broken pipe */
	EPIPE,          
	/* Protocol error */
	EPROTO,         
	/* Protocol not supported */
	EPROTONOSUPPORT,
	/* Protocol wrong type for socket */
	EPROTOTYPE,     
	/* Read-only file system */
	EROFS,          
	/* Invalid seek */
	ESPIPE,         
	/* No such process */
	ESRCH,          
	/* Stream ioctl() timeout */
	ETIME,          
	/* Connection timed out */
	ETIMEDOUT,      
	/* Text file busy */
	ETXTBSY,        
	/* Operation would block */
	EWOULDBLOCK,    
	/* Cross-device link */
	EXDEV,          
};

#endif
