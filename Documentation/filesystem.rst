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

To keep copying to a minimul, memory pages of the requesting/responding
process are mapped into that of the responding/requesting process. When
a process is done with a range of memory pages it must free them themselves.


Requesting processes
''''''''''''''''''''

To send requests, a process uses two ring buffers with identical size:

* A *request queue* (RQ)

* A *completion queue* (CQ)

The size of both queues are always a power of 2 so that wrapping the
head and ``tail`` can be performed with a bitwise ``and`` operation.

Each queue begins with both the ``head`` and ``tail`` which are both
``usize`` s. The entries come after these two fields.


A request is a struct with the following fields:

* An ``u8`` ``opcode`` field, which describes the operation to be performed.

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

The structure is 40 bytes large on 64-bit and 24 bytes large on 32-bit
systems.


When a request has finished, an entry will be added to the CQ which has
the following fields:

* A ``data`` field, which is a union of;

  * A ``*mut mem_page`` or ``*const mem_page`` ``buffer`` field, which may
    be ``null`´ depending on the operation.

  * An ``u32`` ``file_handle``.

* An ``usize`` ``length`` field indicating the actual amount of data read or
  written.

* An ``u32`` ``status`` code indicating whether the operation has succeeded
  or an error occured. The exact value of ``status`` depends on the operation.

* An ``usize`` ``userdata`` field, which is identical to that of the
  corresponding request.

This structure is 32 bytes large on 64-bit and 16 bytes on 32-bit systems.


To send a request, the operation goes as follows:

1. Write out the request.

2. Execute a memory fence.

3. Increment the ``tail`` and wrap if necessary.

The memory fence is necessary so that the ``tail`` won't be updated until
all the fields of the RQ entry have been written out.


New entries in the CQ can be detected by comparing the ``head`` and the
``tail``.


Responding processes
''''''''''''''''''''

To receive requests, a process uses two ring buffers with identical size:

* A *request (input) queue* (IQ)

* A *response (output) queue* (OQ)

The size of both queues are always a power of 2 so that wrapping the
head and ``tail`` can be performed with a bitwise ``and`` operation.

Each queue begins with both the ``head`` and ``tail`` which are both
``usize`` s. The entries come after these two fields.


An IQ entry has the following fields:

* An ``u8`` ``opcode`` field, which describes the operation to be performed.

* An ``u8`` ``priority`` field.

* An ``u16`` ``flags`` field.

* A ``object`` field, which is a union of;

  * An ``usize`` ``file_handle`` field, which describes the object to perform
  the operation on.

  * A ``*const str`` ``name`` field.

* An ``usize`` ``offset`` field.

* A ``data`` field, which is a union of;

  * A ``*mut mem_page`` or ``*const mem_page`` ``buffer`` field.

  * A ``*const small_str`` field.

* An ``usize`` ``length`` field.

* An ``usize`` ``id`` field, which can be used to keep track of requests.

The structure is 40 bytes large on 64-bit and 24 bytes large on 32-bit
systems.

It is identical to the requesting process' RQ entry bar the ``userdata``
field, which is exluded and replaced with an ``id`` field to prevent info
leaks and simplify the kernel implementation.


An OQ entry has the following fields:

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

Again, it is largely identical to that of the requesting process' CQ entry
bar the ``userdata`` / ``id`` field.


To send a response, the operation goes as follows:

1. Write out the reponse.

2. Execute a memory fence.

3. Increment the ``tail`` and wrap if necessary.

The memory fence is necessary so that the ``tail`` won't be updated until
all the fields of the OQ entry have been written out.


To mark a request as accepted, the ``head`` should be incremented and wrapped
if necessary. It may be desireable to copy the IQ entry to a separate buffer
to prevent stalling on a slow request.


Operations
~~~~~~~~~~

Listing
'''''''

+-------------------------+------+
|        Operation        | Code |
+=========================+======+
| READ_                   |   xx |
+-------------------------+------+
| WRITE_                  |   xx |
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

