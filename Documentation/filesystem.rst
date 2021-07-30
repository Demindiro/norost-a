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

To communicate, a task uses two ring buffers with identical size:

* A *transmit queue* (TXQ)

* A *receive queue* (RXQ)

The size of both queues are always a power of 2 so that wrapping the index
can be performed with a bitwise ``and`` operation.


Transmit queue
''''''''''''''

A TXQ entry is a struct with the following fields:

* A ``u8`` ``opcode`` field, which describes the operation to be performed.
  If this field is ``0``, it marks the end of entries to be processed.

* A ``u8`` ``priority`` field.

* A ``u16`` ``flags`` field.

* A ``u32`` ``id`` field, which can be used to keep track of requests.

* A ``tid`` ``address`` field, which describes the task that should receive
  the request.

* A ``usize`` ``length`` field, which describes the amount of data to be read or
  written.

* A ``data`` field, which is a pointer to an arbitrary blob of data. The
  format of the data depends on the flags.

The fields must be in the given order and be properly aligned.

To send data, the operation goes as follows:

1. Write out the structure **without** the ``opcode``.

2. Execute a memory fence.

3. Write out the ``opcode``.

The memory fence is necessary so that the ``opcode`` won't be written until
all the fields of the RQ entry have been written out.


Receive queue
'''''''''''''

An RXQ entry is identical to a TXQ entry except that ``address`` corresponds
to that of the sending task instead of the receiving task.

To send a response, a TXQ structure is filled out with the ``id`` and
``address`` matching that of the RXQ structure.


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

