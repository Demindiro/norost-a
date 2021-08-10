=============
Notifications
=============

Notifications are small packets that can be sent by either the kernel or
another task. While this is a form of IPC, it serves a different purpose and
hence is documented separately.

The main difference is the ability to *interrupt* a task, i.e. it will suspend
the main routine and begins running another routine specified by
``io_set_interrupt_handler``. This makes it useful for handling timers,
establishing soft real-time communication and handling potentially fatal errors
such as accessing invalid memory.

Additionaly, notifications are *synchronous*, i.e. they are immediately
processed the moment they are sent.


Packet format
~~~~~~~~~~~~~

A packet has the following fields:

* An ``tid`` ``address`` address fields, which specifies the source of the
  packet. An ``address`` of ``-1`` is reserved for the kernel.

* A ``usize`` ``type`` field, which indicates what the ``value`` represents.

* A ``usize`` ``value`` field. A ``usize`` is large enough for a single
  pointer, which may be useful if the notification pertains to shared memory.


Kernel messages
~~~~~~~~~~~~~~~


Table
'''''

+----+-----------------------+
| ID | Type                  |
+----+-----------------------+
|  0 | `External Interrupt`_ |
+----+-----------------------+


Descriptions
''''''''''''

External Interrupt
``````````````````

An interrupt emitted by an external source was caught.

To pick these interrupts up, its ID needs to be specified using
``io_add_interrupt_listener``. These can be removed afterwards using
``io_remove_interrupt-listener``.

To mark an interrupt as completed, ``io_complete_interrupt`` should be called.
