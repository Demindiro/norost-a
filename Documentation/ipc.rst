==================================
Inter Process / Task Communication
==================================

All communication is stateless: there is no need to allocate some object before
communicating with another task. Instead, each packet has an address field
indicating the recipient. The address field is simply the TID of the task that
should receive the packet.

To prevent excessive blocking, all communication is asynchronous: packets to be
sent are put in a *transmit queue* and received packets are put in a *receive
queue*. Explicit synchronization can be achieved with the ``io_wait`` syscall.

To avoid copying overhead, data is sent by sharing pages between tasks.


Userland implementation
~~~~~~~~~~~~~~~~~~~~~~~

Transmit queue
''''''''''''''

A TXQ entry is a struct with the following fields:

* A ``UUID`` ``uuid`` field to identify an object.

* A ``*mut _``data`` field, which is a pointer to an arbitrary blob of data. The
  format of the data depends on the flags.

* A ``usize`` ``length`` field, which describes the amount of data to be read or
  written.

* A ``u64`` ``offset`` field that indicates an offset inside the object.

* A ``tid`` ``address`` field, which describes the task that should receive
  the request.

* A ``u16`` ``flags`` field.

* A ``u8`` ``opcode`` field, which describes the operation to be performed.
  If this field is ``0``, it marks the end of entries to be processed.

* A ``u8`` ``id`` field, which can be used to differentiate multiple requests
  for the same object.

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

This table defines how *user* applications should interpret. User applications
are allowed to interpret and/or add custom operations, although this is not
recommended.

Note that ``flags`` have to be defined appropriately for each operation to
behave as expected.

Listing
'''''''

+-------------------------+------+
|        Operation        | Code |
+=========================+======+
| READ_                   |    1 |
+-------------------------+------+
| WRITE_                  |    2 |
+-------------------------+------+
| INFO_                   |    3 |
+-------------------------+------+
| LIST_                   |    4 |
+-------------------------+------+
| MAP_READ_               |    5 |
+-------------------------+------+
| MAP_WRITE_              |    6 |
+-------------------------+------+
| MAP_READ_WRITE_         |    7 |
+-------------------------+------+
| MAP_EXEC_               |    8 |
+-------------------------+------+
| MAP_READ_EXEC_          |    9 |
+-------------------------+------+
| MAP_READ_COW_           |   10 |
+-------------------------+------+
| MAP_EXEC_COW_           |   11 |
+-------------------------+------+
| MAP_READ_EXEC_COW_      |   12 |
+-------------------------+------+


Descriptions
''''''''''''

READ
````

Read data at an offset from an object into the given memory pages.

The offset is ignored if it does not apply (e.g. TCP sockets).


WRITE
`````

Write data from the given memory pages into an object at an offset.

The offset is ignored if it does not apply (e.g. TCP sockets).


INFO
````

Write a structure into the given memory page that describes the object.


LIST
````

Write a structure into the given memory page that lists any child objects
this object may have.

The structure is an array containing a list of object entries. Each entry
has the following fields:

* ``UUID`` ``uuid``

* ``u32`` ``name_offset``

* ``u16`` ``name_length``

The ``name_offset`` field points to a string relative to the starting address
of the data. If the object has no name, it should be 0.


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


MAP_READ_EXEC_COW
`````````````````

Returns a read & execute page range that maps a section of an object.

This range will not be affected by writes to other mappings. Existence or
creation of a writeable range will cause a new page range to be allocated.
