==========
Filesystem
==========


What is a file?
~~~~~~~~~~~~~~~

Definition: *A collection of data*.

By this definition a file is anything from which data can be read and/or data
can be written to.


Implementation
~~~~~~~~~~~~~~

File I/O is IPC in reality:

* To read from a file, a process requests data from another process which
  may manage one or more devices.

* To write to a file, a process passes data to another process which may
  manage one or more devices.

To keep copying to a minimum, memory pages of the client/server task are
mapped into that of the server/client task. When a task is done with a range
of memory pages it must free them themselves.


Client tasks
''''''''''''

To send requests, a task uses two ring buffers with identical size:

* A *request queue* (CRQ)

* A *completion queue* (CCQ)

The size of both queues are always a power of 2 so that wrapping the index
can be performed with a bitwise ``and`` operation.


A CRQ entry is a struct with the following fields:

* An ``u8`` ``opcode`` field, which describes the operation to be performed.
  If this field is ``0``, it marks the end of entries to be processed.

* An ``u8`` ``priority`` field.

* An ``u16`` ``flags`` field.

* An ``u32`` ``file_handle`` field, which describes the object to perform
  the operation on.

* An ``usize`` ``offset`` field.

* A ``data`` field, which is a union of;

  * A ``*mut mem_page`` or ``*const mem_page`` ``buffer`` field.

  * A ``*const small_str`` field.

* An ``usize`` ``length`` field.

* An ``usize`` ``userdata`` field, which can be used to keep track of
  requests.

The structure is 64 bytes large on 64-bit and 32 bytes large on 32-bit
systems.


When a request has finished, an entry will be added to the CCQ which has
the following fields:

* A ``data`` field, which is a union of;

  * A ``*mut mem_page`` or ``*const mem_page`` ``buffer`` field, which may
    be ``null`` depending on the operation.

  * An ``u32`` ``file_handle``.

* An ``usize`` ``length`` field indicating the actual amount of data read or
  written.

* An ``u32`` ``status`` code indicating whether the operation has succeeded
  or an error occured. The exact value of ``status`` depends on the operation.

* An ``usize`` ``userdata`` field, which is identical to that of the
  corresponding request.

This structure is 32 bytes large on 64-bit and 16 bytes on 32-bit systems.

The kernel does not check whether a completion entry has been processed by
the task. It is up to the task to prevent overwriting existing entries.


To send a request, the operation goes as follows:

1. Write out the request **without** the ``opcode``.

2. Execute a memory fence.

3. Write out the ``opcode``.

The memory fence is necessary so that the ``opcode`` won't be written until
all the fields of the RQ entry have been written out.


New entries can be detected by any application-specific method. Using ``0``
for the ``userdata`` field to indicate empty entries is the conventional
approach.


Server tasks
''''''''''''

To receive requests, a task uses two ring buffers with identical size:

* A *request queue* (SRQ)

* A *completion queue* (SCQ)

The size of both queues are always a power of 2 so that wrapping the
index can be performed with a bitwise ``and`` operation.

Each queue begins with both the ``head`` and ``tail`` which are both
``usize`` s. The entries come after these two fields.


An SRQ entry has the following fields:

* An ``u8`` ``opcode`` field, which describes the operation to be performed.
  If it is ``0``, the entry is empty.

* An ``u8`` ``priority`` field.

* An ``u16`` ``flags`` field.

* A ``object`` field, which is a union of;

  * An ``usize`` ``file_handle`` field, which describes the object to perform
  the operation on.

  * A ``*const small_str`` ``name`` field.

  * A ``*const small_[u8]`` ``uuid`` field.

* An ``usize`` ``offset`` field.

* A ``data`` field, which is a union of;

  * A ``*mut mem_page`` or ``*const mem_page`` ``buffer`` field.

  * A ``*const small_str`` field.

* An ``usize`` ``length`` field.

* An ``usize`` ``id`` field, which can be used to keep track of requests.
  This field is never ``0``.

The structure is 64 bytes large on 64-bit and 32 bytes large on 32-bit
systems.

It is identical to the requesting process' CRQ entry bar the ``userdata``
field, which is excluded and replaced with an ``id`` field to prevent info
leaks and simplify the kernel implementation.


A SCQ entry has the following fields:

* A ``data`` field, which is a union of;

  * A ``*mut mem_page`` or ``*const mem_page`` ``buffer`` field, which may
    be ``null`` depending on the operation.

  * An ``usize`` ``file_handle``.

* An ``usize`` ``length`` field indicating the actual amount of data read or
  written.

* An ``u32`` ``status`` code indicating whether the operation has succeeded
  or an error occured. The exact value of ``status`` depends on the operation.

* An ``usize`` ``id`` field, which is identical to that of the
  corresponding request.


This structure is 32 bytes large on 64-bit and 16 bytes on 32-bit systems.

Again, it is largely identical to that of the requesting task's CCQ entry
bar the ``userdata`` / ``id`` field.

A ``0`` ``id`` field means the entry is empty.


To send a response, the operation goes as follows:

1. Write out the entry **without** the ``id`` field.

2. Execute a memory fence.

3. Write out the ``id`` field.

The memory fence is necessary so that the ``tail`` won't be updated until
all the fields of the OQ entry have been written out.




Operations
~~~~~~~~~~

Listing
'''''''

+-------------------------+------+
|        Operation        | Code |
+=========================+======+
| READ_                   |   xx |
+-------------------------+------+
| WRITE_                  |    2 |
+-------------------------+------+
| OPEN_                   |   xx |
+-------------------------+------+
| CLOSE_                  |   xx |
+-------------------------+------+
| INFO_                   |   xx |
+-------------------------+------+
| MAP_READ_               |   xx |
+-------------------------+------+
| MAP_WRITE_              |   xx |
+-------------------------+------+
| MAP_READ_WRITE_         |   xx |
+-------------------------+------+
| MAP_EXEC_               |   xx |
+-------------------------+------+
| MAP_READ_EXEC_          |   xx |
+-------------------------+------+
| MAP_READ_COW_           |   xx |
+-------------------------+------+
| MAP_EXEC_COW_           |   xx |
+-------------------------+------+
| MAP_READ_EXEC_COW_      |   xx |
+-------------------------+------+
| READ_ONCE_              |   xx |
+-------------------------+------+
| WRITE_ONCE_             |   xx |
+-------------------------+------+
| INFO_ONCE_              |   xx |
+-------------------------+------+
| MAP_READ_ONCE_          |   xx |
+-------------------------+------+
| MAP_WRITE_ONCE_         |   xx |
+-------------------------+------+
| MAP_READ_WRITE_ONCE_    |   xx |
+-------------------------+------+
| MAP_EXEC_ONCE_          |   xx |
+-------------------------+------+
| MAP_READ_EXEC_ONCE_     |   xx |
+-------------------------+------+
| MAP_READ_COW_ONCE_      |   xx |
+-------------------------+------+
| MAP_EXEC_COW_ONCE_      |   xx |
+-------------------------+------+
| MAP_READ_EXEC_COW_ONCE_ |   xx |
+-------------------------+------+


Descriptions
''''''''''''

READ
````

Read data at an offset from an object into the given memory pages.

The offset is ignored if it does not apply (e.g. TCP sockets).


WRITE
`````

Write data from the given memory pages into from an object at an offset.

The offset is ignored if it does not apply (e.g. TCP sockets).


OPEN
````

Map an object to a file handle and return the handle.


CLOSE
`````

Destroy the handle mapping to an object.


INFO
````

Write a structure into the given memory page that describes the object.


MAP_READ
````````

Returns a read-only page range that maps a section of an object.

This range may be affected by writes to other mappings.


MAP_WRITE
`````````

Returns a write-only page range that maps a section of an object.

This range may be affected by writes to other mappings.


MAP_READ_WRITE
``````````````

Returns a read & write page range that maps a section of an object.

This range may be affected by writes to other mappings.


MAP_EXEC
````````

Returns a execute-only page range that maps a section of an object.

This range may be affected by writes to other mappings.


MAP_READ_EXEC
`````````````

Returns a read & execute page range that maps a section of an object.

This range may be affected by writes to other mappings.


MAP_READ_COW
`````````````

Returns a read-only page range that maps a section of an object.

This range will not be affected by writes to other mappings. Existence or
creation of a writeable range will cause a new page range to be allocated.


MAP_EXEC_COW
````````````

Returns a execute-only page range that maps a section of an object.

This range will not be affected by writes to other mappings. Existence or
creation of a writeable range will cause a new page range to be allocated.


MAP_READ_EXEC
`````````````

Returns a read & execute page range that maps a section of an object.

This range will not be affected by writes to other mappings. Existence or
creation of a writeable range will cause a new page range to be allocated.


READ_ONCE
`````````

Same as READ_ but does not allocate a file handle.


WRITE_ONCE
``````````

Same as WRITE_ but does not allocate a file handle.


INFO_ONCE
`````````

Same as INFO_ but does not allocate a file handle.


MAP_READ_ONCE
`````````````

Same as MAP_READ_ but does not allocate a file handle.


MAP_WRITE_ONCE
``````````````

Same as MAP_WRITE_ but does not allocate a file handle.


MAP_READ_WRITE_ONCE
```````````````````

Same as MAP_READ_WRITE_ but does not allocate a file handle.


MAP_EXEC_ONCE
`````````````

Same as MAP_EXEC_ but does not allocate a file handle.


MAP_READ_EXEC_ONCE
``````````````````

Same as MAP_READ_EXEC_ but does not allocate a file handle.


MAP_READ_COW_ONCE
`````````````````

Same as MAP_READ_COW_ but does not allocate a file handle.


MAP_EXEC_COW_ONCE
`````````````````

Same as MAP_EXEC_COW_ but does not allocate a file handle.


MAP_READ_EXEC_COW_ONCE
``````````````````````

Same as MAP_READ_EXEC_COW_ but does not allocate a file handle.

