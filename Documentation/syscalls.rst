============
System calls
============

System calls allow communication with the priviliged layers of the OS. By
extension, this also allows requesting resources.

System calls may take up to 4 arguments and return up to two parameters.

If an argument doesn't fit within a single register, it is split up into two
registers. If the argument doesn't fit within two registers, a pointer to
the argument is passed instead. Arguments that may span multiple registers
are always last.

ABI
~~~

+----------------+----+----+----+----+----+----+----+
| Architecture   | ID | a0 | a1 | a2 | a3 | r0 | r1 |
+================+====+====+====+====+====+====+====+
| RISC-V (RV32I) | a7 | a0 | a1 | a2 | a3 | a0 | a1 |
+----------------+----+----+----+----+----+----+----+
| RISC-V (RV64I) | a7 | a0 | a1 | a2 | a3 | a0 | a1 |
+----------------+----+----+----+----+----+----+----+


Listing
~~~~~~~

+------------------------+----+
|          Call          | ID |
+========================+====+
| io_wait_               |  0 |
+------------------------+----+
| io_set_client_buffers_ |  1 |
+------------------------+----+
| io_set_server_buffers_ |  2 |
+------------------------+----+
| mem_alloc_             |  3 |
+------------------------+----+
| mem_dealloc_           |  4 |
+------------------------+----+
| mem_get_flags_         |  5 |
+------------------------+----+
| mem_set_flags_         |  6 |
+------------------------+----+
| mem_physical_address_  |  7 |
+------------------------+----+
| task_id_               | xx |
+------------------------+----+
| task_yield_            | xx |
+------------------------+----+
| task_sleep_            | xx |
+------------------------+----+
| task_spawn_            | xx |
+------------------------+----+
| task_destroy_          | xx |
+------------------------+----+
| task_suspend_          | xx |
+------------------------+----+
| sys_direct_alloc_      | xx |
+------------------------+----+


Descriptions
~~~~~~~~~~~~

io_wait
'''''''

+--------+-----------------------------+-----------------------+
| **ID** |                          xx |                       |
+--------+-----------------------------+-----------------------+
| **a0** | ``u8``                      | ``flags``             |
+--------+-----------------------------+-----------------------+
| **a1** | ``u64``                     | ``time``              |
+--------+-----------------------------+-----------------------+
| **r0** | ``io_ring_wait_status``     | ``status``            |
+--------+-----------------------------+-----------------------+

Halts the calling task until an I/O event occurs.

Valid ``flags`` are:

* ``IO_WAIT_ALL`` (``0x1``): Wait for all events to complete.

* ``IO_WAIT_REQUESTER`` (``0x2``): Wait for requester events.

* ``IO_WAIT_RESPONDER`` (``0x4``): Wait for responder events.

* ``IO_WAIT_MAX_TIME`` (``0x8``): Wait only for a certain amount of time.


io_set_client_buffers
'''''''''''''''''''''

+--------+----------------------------+----------------------------+
| **ID** |                         xx |                            |
+--------+----------------------------+----------------------------+
| **a0** | ``*mut io_ring_crq``       | ``request_buffer``         |
+--------+----------------------------+----------------------------+
| **a1** | ``u8``                     | ``size``                   |
+--------+----------------------------+----------------------------+
| **a3** | ``*mut io_ring_ccq``       | ``completion_buffer``      |
+--------+----------------------------+----------------------------+
| **a4** | ``u8``                     | ``size``                   |
+--------+----------------------------+----------------------------+
| **r0** | ``io_ring_create_status``  | ``status``                 |
+--------+----------------------------+----------------------------+

Sets the buffers to be used for client request and completion entries.

``size`` is the power of the of the size, i.e. the actual size is
``pow(2, size)``.


io_resize_responder
'''''''''''''''''''

+--------+------------------------------+----------------------------+
| **ID** |                           xx |                            |
+--------+------------------------------+----------------------------+
| **a0** | ``*mut io_ring_responder``   | ``request_buffer``         |
+--------+------------------------------+----------------------------+
| **a1** | ``usize``                    | ``size``                   |
+--------+------------------------------+----------------------------+
| **r0** | ``io_ring_create_status``    | ``status``                 |
+--------+------------------------------+----------------------------+

Resizes the ``io_ring_responder`` buffer for this task.

A ``io_ring_reponder`` has the following fields:

* A ``*mut io_ring_repond_in`` ``respond_in``. If this is ``null``, the kernel
  will pick an address. Otherwise, the kernel will attempt to map the
  buffer to this address.

* A ``usize`` ``repond_in_head``, which is an *unmasked* index of the head.

* A ``usize`` ``repond_in_tail``, which is an *unmasked* index of the tail.

* A ``*mut io_ring_respond_out`` ``responses``. If this is ``null``, the kernel
  will pick an address. Otherwise, the kernel will attempt to map the
  buffer to this address.

* A ``usize`` ``respond_out_head``, which is an *unmasked* index of the head.

* A ``usize`` ``respond_out_tail``, which is an *unmasked* index of the tail.

``size`` must be a power of two.


mem_alloc
'''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``*const mem_page``       | ``virtual_address``        |
+--------+---------------------------+----------------------------+
| **a1** | ``usize``                 | ``count``                  |
+--------+---------------------------+----------------------------+
| **a2** | ``u8``                    | ``flags``                  |
+--------+---------------------------+----------------------------+
| **r0** | ``mem_alloc_status``      | ``status``                 |
+--------+---------------------------+----------------------------+
| **r1** | ``*const mem_page``       | ``allocation``             |
+--------+---------------------------+----------------------------+

Allocate ``count`` pages. The allocated pages will be mapped to
``virtual_address``.

``virtual_address`` must be properly aligned.

Valid flags are:

* ``PROTECT_ALLOW_READ`` (``0x1``): Allow reading the pages.

* ``PROTECT_ALLOW_WRITE`` (``0x2``): Allow writing the pages.

* ``PROTECT_ALLOW_EXECUTE`` (``0x4``): Allow fetching and executing
  instructions from the pages.

* ``SHAREABLE`` (``0x8``): Allow sharing the pages with other tasks.

* ``SIZE_MEGAPAGE`` (``0x10``): Allocate a megapage. The size and alignment
  is architecture-dependent.

* ``SIZE_GIGAPAGE`` (``0x20``): Allocate a gigapage. The size and alignment
  is architecture-dependent.

* ``SIZE_TERAPAGE`` (``0x30``): Allocate a terapage. The size and alignment
  is architecture-dependent.


The pages are guaranteed to be zeroed.

Possible errors are:

* ``INVALID_FLAGS`` (``1``): The combination of protection flags is not
  supported.

* ``OVERLAP`` (``2``): The address range overlaps with an existing range.

* ``


mem_dealloc
'''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``*const mem_page``       | ``virtual_address``        |
+--------+---------------------------+----------------------------+
| **a1** | ``usize``                 | ``count``                  |
+--------+---------------------------+----------------------------+
| **r0** | ``mem_dealloc_status``    | ``status``                 |
+--------+---------------------------+----------------------------+

Deallocates a range of pages starting from the given address. The address must
be properly aligned.


mem_get_flags
'''''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``*const mem_page``       | ``virtual_address``        |
+--------+---------------------------+----------------------------+
| **r0** | ``mem_get_flags_status``  | ``status``                 |
+--------+---------------------------+----------------------------+

Get the flags of the given page. The flags are shared between all pages of
an allocation.


mem_set_flags
'''''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``*const mem_page``       | ``virtual_address``        |
+--------+---------------------------+----------------------------+
| **r0** | ``mem_set_flags_status``  | ``status``                 |
+--------+---------------------------+----------------------------+

Set the flags of the given page. The flags are shared between all pages of
an allocation.


mem_physical_address
''''''''''''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``*const mem_page``       | ``virtual_address``        |
+--------+---------------------------+----------------------------+
| **a1** | ``*mut mem_ppn``          | ``physical_page_numbers``  |
+--------+---------------------------+----------------------------+
| **a2** | ``usize``                 | ``count``                  |
+--------+---------------------------+----------------------------+
| **r0** | ``mem_set_flags_status``  | ``status``                 |
+--------+---------------------------+----------------------------+

Return the physical page numbers backing a virtual address range.


task_id
'''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **r1** | ``usize``                 | ``size``                   |
+--------+---------------------------+----------------------------+

Return the ID of the current task. This call cannot fail.


task_yield
''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **r0** | ``task_yield_status``     | ``status``                 |
+--------+---------------------------+----------------------------+

Yield control to let any other task run.


task_sleep
''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``u64``                   | ``time``                   |
+--------+---------------------------+----------------------------+
| **r0** | ``task_sleep_status``     | ``status``                 |
+--------+---------------------------+----------------------------+

Suspend the task for the given amount of ``nanoseconds``.


task_spawn
''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``*const new_task``       | ``task_info``              |
+--------+---------------------------+----------------------------+
| **r0** | ``task_spawn_status``     | ``status``                 |
+--------+---------------------------+----------------------------+
| **r1** | ``usize``                 | ``task_id``                |
+--------+---------------------------+----------------------------+

Create a new task with the given file handles, memory pages and user ID
and starts at the ``entry`` point.

The ``new_task`` struct has the following fields:

* ``usize`` ``user_id``.  If ``user_id`` is ``0``, the current UID will
  be used for the new task. Otherwise, if the current UID is ``0`` (i.e.
  ``root``) the task will be assigned the new UID. If it is not ``0``,
  ``NO_PERMISSION`` will be returned if it doesn't match the current UID.

* ``u8`` ``flags`` with the following flags:

  * ``SHARE_RESOURCES`` (``0x1``): The new task will share the same resources
    as that of the current task, which includes memory pages and file handles.
    i.e. if one of both tasks allocates a new memory page / file handle, it
    will also be accessible for the other task. The ``memory_pages`` and
    ``file_handles`` fields will be ignored.

* ``usize`` ``memory_pages_count``

* ``*const mem_page`` ``memory_pages``

* ``usize`` ``file_handles_count``

* ``*const u32`` ``file_handles``. Each entry in ``file_handles`` moves a file
  handle out of the current task and assigns it to the new task. The new file
  handle's ID is the index in the array.


task_destroy
''''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``usize``                 | ``task_id``                |
+--------+---------------------------+----------------------------+
| **a1** | ``u8``                    | ``reason``                 |
+--------+---------------------------+----------------------------+
| **r0** | ``task_destroy_status``   | ``status``                 |
+--------+---------------------------+----------------------------+


task_suspend
''''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``usize``                 | ``task_id``                |
+--------+---------------------------+----------------------------+
| **a1** | ``u8``                    | ``reason``                 |
+--------+---------------------------+----------------------------+
| **r0** | ``task_destroy_status``   | ``status``                 |
+--------+---------------------------+----------------------------+


sys_direct_alloc
''''''''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``*const mem_page``       | ``virtual_address``        |
+--------+---------------------------+----------------------------+
| **a1** | ``usize``                 | ``physical_page_number``   |
+--------+---------------------------+----------------------------+
| **a2** | ``usize``                 | ``page_count``             |
+--------+---------------------------+----------------------------+
| **r0** | ``task_destroy_status``   | ``status``                 |
+--------+---------------------------+----------------------------+

Directly maps a range of physical addresses into the task's address space. This
call is very dangerous and may only be used by drivers.

Note that the call accepts **page numbers**, not addresses!


Error codes
~~~~~~~~~~~

To keep implementation and debugging simple, some of the error codes are
shared between system calls. The table below lists the code of each error.

+----------------------+----+--------------------------------------------------+
| Error                | ID | Description                                      |
+======================+====+==================================================+
| OK                   |  0 | No error.                                        |
+----------------------+----+--------------------------------------------------+
| INVALID_CALL         |  1 | The call doesn't exist.                          |
+----------------------+----+--------------------------------------------------+
| NULL_ARGUMENT        |  2 | One of the arguments is ``null`` when it         |
|                      |    | shouldn't be.                                    |
+----------------------+----+--------------------------------------------------+
| MEM_OVERLAP          |  3 | The address range overlaps with another range.   |
+----------------------+----+--------------------------------------------------+
| MEM_UNAVAILABLE      |  4 | There is no more memory available.               |
+----------------------+----+--------------------------------------------------+
| MEM_LOCKED           |  5 | The flags of one or more memory pages are        |
|                      |    | locked.                                          |
+----------------------+----+--------------------------------------------------+
| MEM_NOT_ALLOCATED    |  6 | The memory at the address is no allocated, i.e.  |
|                      |    | it doesn't exist.                                |
+----------------------+----+--------------------------------------------------+
| MEM_INVALID_PROTECT  |  7 | The combination of memory protection flags isn't |
|                      |    | supported.                                       |
+----------------------+----+--------------------------------------------------+
| MEM_BAD_ALIGNMENT    |  8 | The address isn't properly aligned.              |
+----------------------+----+--------------------------------------------------+
| IO_MEM_NOT_SHAREABLE | xx | The memory cannot be shared between tasks as it  |
|                      |    | is private memory.                               |
+----------------------+----+--------------------------------------------------+
