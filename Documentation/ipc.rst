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


Implementation
~~~~~~~~~~~~~~

Packet table
''''''''''''

Communication is achieved with the use of packets. Packets are stored in a
contiguous array.

Packets to be sent are put in the transmit queue, which is a ring buffer of
packet slots.

Packets that are received are put in the send queue by the kernel, which is
also a ring buffer.

Free packet slots are put in the free stack. This stack is used by both the
kernel

The structure is layed out as follows:

::
   
   +-------------------+
   | packet 0          |
   | packet 1          |
   | ...               |
   | packet n-1        |
   +-------------------+
   | transmit index    |
   +-------------------+
   | transmit slot 0   |
   | transmit slot 1   |
   | ...               |
   | transmit slot n-1 |
   +-------------------+
   | received index    |
   +-------------------+
   | received slot 0   |
   | ...               |
   +-------------------+
   | free stack index  |
   +-------------------+
   | free slot 0       |
   | ...               |
   +-------------------+

At most ``2 ^ 15`` packets can be allocated, as slots are addressed with
16-bit unsigned integers and to prevent ambiguity when the kernel/client side
``index`` is equal to that of the ring.


Packets
'''''''

A packet has the following fields:

* A ``UUID`` ``uuid`` field to identify an object.

* A ``*mut Page``data`` field, which is the start address of a page range.

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


Flags
`````

+-----+-------------+-------------------------------------------------------+
| Bit | Name        | Description                                           |
+-----+-------------+-------------------------------------------------------+
|   0 | Readable    | Make data pages readable                              |
+-----+-------------+-------------------------------------------------------+
|   1 | Writeable   | Make data pages writeable                             |
+-----+-------------+-------------------------------------------------------+
|   2 | Executable  | Make data pages executable                            |
+-----+-------------+-------------------------------------------------------+
|   3 | Lock        | Lock page flags                                       |
+-----+-------------+-------------------------------------------------------+
|   4 | Response    | The packet is a response                              |
+-----+-------------+-------------------------------------------------------+
|   5 | Error       | The packet is an error response                       |
+-----+-------------+-------------------------------------------------------+
|   6 | Quiet       | No confirmation of successfull processing is expected |
+-----+-------------+-------------------------------------------------------+
|   7 | Unavailable | The resource is unavailable                           |
+-----+-------------+-------------------------------------------------------+


Transmitting packets
''''''''''''''''''''

To send data, the operation goes as follows:

1. Write out the structure.

2. Add the slot index to the ``transmit`` ring buffer..

3. Execute a memory fence.

4. Increment the transmit ring index.


The memory fence is necessary so that the ``opcode`` won't be written until
all the fields of the RQ entry have been written out.


Receiving packets
'''''''''''''''''

To detect received packets, a private ``last_received`` index should be used.
If this index is *not* equal to that of the ``received`` ring buffer, it should
be incremented until it does. Any elements in between are slots of new received
packets.


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
