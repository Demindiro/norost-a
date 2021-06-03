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
| io_wait_               | xx |
+------------------------+----+
| io_resize_requester_   | xx |
+------------------------+----+
| io_resize_responder_   | xx |
+------------------------+----+
| mem_alloc_             | xx |
+------------------------+----+
| mem_alloc_shared_      | xx |
+------------------------+----+
| mem_dealloc_           | xx |
+------------------------+----+
| mem_alloc_range_       | xx |
+------------------------+----+
| mem_dealloc_range_     | xx |
+------------------------+----+
| mem_get_flags_         | xx |
+------------------------+----+
| mem_set_flags_         | xx |
+------------------------+----+
| dev_reserve_           | xx |
+------------------------+----+
| dev_release_           | xx |
+------------------------+----+
| dev_list_              | xx |
+------------------------+----+
| dev_info_              | xx |
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
| task_exit_             | xx |
+------------------------+----+
| task_signal_           | xx |
+------------------------+----+
| task_signal_handler_   | xx |
+------------------------+----+
| task_pin_cpu_          | xx |
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


io_resize_requester
'''''''''''''''''''

+--------+----------------------------+----------------------------+
| **ID** |                         xx |                            |
+--------+----------------------------+----------------------------+
| **a0** | ``*mut io_ring_requester`` | ``request_buffer``         |
+--------+----------------------------+----------------------------+
| **a1** | ``usize``                  | ``size``                   |
+--------+----------------------------+----------------------------+
| **r0** | ``io_ring_create_status``  | ``status``                 |
+--------+----------------------------+----------------------------+

Resizes the ``io_ring_requester`` buffer for this task.

A ``io_ring_requester`` has the following fields:

* A ``*mut io_ring_request`` ``requests``. If this is ``null``, the kernel
  will pick an address. Otherwise, the kernel will attempt to map the
  buffer to this address.

* A ``usize`` ``request_head``, which is an *unmasked* index of the head.

* A ``usize`` ``request_tail``, which is an *unmasked* index of the tail.

* A ``*mut io_ring_response`` ``responses``. If this is ``null``, the kernel
  will pick an address. Otherwise, the kernel will attempt to map the
  buffer to this address.

* A ``usize`` ``response_head``, which is an *unmasked* index of the head.

* A ``usize`` ``response_tail``, which is an *unmasked* index of the tail.

``size`` must be a power of two.


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


dev_reserve
'''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``*mut mem_page``         | ``virtual_address``        |
+--------+---------------------------+----------------------------+
| **a1** | ``usize``                 | ``device_id``              |
+--------+---------------------------+----------------------------+
| **a2** | ``u8``                    | ``flags``                  |
+--------+---------------------------+----------------------------+
| **r0** | ``dev_reserve_status``      | ``status``                 |
+--------+---------------------------+----------------------------+

Map the device with the ``device_id`` to the ``virtual_address``.


dev_release
'''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``*mut mem_page``         | ``virtual_address``        |
+--------+---------------------------+----------------------------+
| **r0** | ``dev_release_status``    | ``status``                 |
+--------+---------------------------+----------------------------+

Unmap the device allocated at the ``virtual_address``.


dev_list
''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``*mut u32``              | ``out``                    |
+--------+---------------------------+----------------------------+
| **a1** | ``usize``                 | ``count``                  |
+--------+---------------------------+----------------------------+
| **a2** | ``usize``                 | ``offset``                 |
+--------+---------------------------+----------------------------+
| **r0** | ``dev_list_status``       | ``status``                 |
+--------+---------------------------+----------------------------+
| **r1** | ``usize``                 | ``total``                  |
+--------+---------------------------+----------------------------+

Return a list of all devices by writing ``count`` IDs to ``out``. Each ID is
a 32-bit unsigned integer. ``total`` indicates the total amount of devices
available.

Each ID is sorted chronologically, so ``òffset`` can reliably be used if a
needed device ID is not present in ``out``.

To only get the total amount of devices, ``count`` can be set to 0 to prevent
writing to ``out``.


dev_info
''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``*mut usize``            | ``out``                    |
+--------+---------------------------+----------------------------+
| **a1** | ``usize``                 | ``out_size``               |
+--------+---------------------------+----------------------------+
| **r0** | ``dev_info_status``       | ``status``                 |
+--------+---------------------------+----------------------------+
| **r1** | ``usize``                 | ``size``                   |
+--------+---------------------------+----------------------------+

Writes info about the device ``device_id`` to ``out``, which must be at
least ``out_size`` bytes large and aligned to a ``usize`` boundary.

On success, ``size`` indicates how many bytes were actually written. On
failure due to an undersized buffer, it indicates how many bytes are needed
to write the information.


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


task_signal
'''''''''''

+--------+---------------------------+----------------------------+
| **ID** |                        xx |                            |
+--------+---------------------------+----------------------------+
| **a0** | ``usize``                 | ``task_id``                |
+--------+---------------------------+----------------------------+
| **a1** | ``u8``                    | ``signal_id``              |
+--------+---------------------------+----------------------------+
| **a2** | ``usize``                 | ``arg0``                   |
+--------+---------------------------+----------------------------+
| **a3** | ``usize``                 | ``arg1``                   |
+--------+---------------------------+----------------------------+
| **r0** | ``task_signal_status``    | ``status``                 |
+--------+---------------------------+----------------------------+

Sends a signal to a task.


task_signal_handler
'''''''''''''''''''

+--------+---------------------------------+--------------------+
| **ID** |                              xx |                    |
+--------+---------------------------------+--------------------+
| **a0** | ``u8``                          | ``signal_id``      |
+--------+---------------------------------+--------------------+
| **a1** | ``*const fn(u8, usize, usize)`` | ``signal_handler`` |
+--------+---------------------------------+--------------------+
| **r0** | ``task_set_handler_status``     | ``status``         |
+--------+---------------------------------+--------------------+
| **r1** | ``*const fn(u8, usize, usize)`` | ``prev_handler``   |
+--------+---------------------------------+--------------------+

Set a handler for a signal. This overrides the default handler.

Passing ``null`` restores the default handler.


