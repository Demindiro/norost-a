==========
Interrupts
==========

One place where this kernel deviates from common microkernel dogma is the
handling of interrupts. Specifically, it includes platform-specific drivers
that could be implemented as userland drivers. This is done because:

* Interrupts are a vital part of every efficient OS.

* Implementing it in userland is both more complicated and likely very slow.


Enabling interrupts
~~~~~~~~~~~~~~~~~~~

Even though the kernel needs access to the PIC, it doesn't detect it
automatically. Instead, a userland driver is supposed to find the location of
the controller and tell the kernel its location via
``sys_set_interrupt_controller``. The amount of parameters depends on the
platform and there is no guarantee it exists.

This is done so that any funky details with device trees, ACPI ... stay outside
the kernel.


Registering interrupts <-> driver mappings
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

To receive interrupts, a driver tells the kernel to register an interrupt
source for itself. The kernel then adds a source -> address mapping. Any time
the kernel receives an interrupt it will send a notification to the driver.


Handling interrupts
~~~~~~~~~~~~~~~~~~~

The exact process by which interrupts are handled is PIC-dependent, but in
general the kernel will mask/claim the corresponding interrupt, send a
notification and then wait for the driver to unmask/complete the interrupt.

One disadvantage of this approach is that other interrupts may get stalled if a
driver is slow to handle an interrupt (be it due inefficies in the driver
itself or due to the kernel not scheduling it quickly). To alleviate this, a
driver can specify a handler that will be ran _immediately_ on receiving the
interrupt. This may happen while the driver itself is already running on
another hart!

.. note::

   This feature requires a context switch, which may impede performance heavily
   if the architecture doesn't support some form of ASID/PCIDs.


Marking an interrupt has handled
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

To indicate a handler has finished running, ``io_interrupt_complete`` should be
called. If using immediate handlers, the interrupt will be marked as completed
as soon as the handler exits.
