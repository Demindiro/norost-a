================
Kernel internals
================

This document describes some of the internals of the kernel in broad terms.
For more specific details, see the source code.


Boot
~~~~

There are 3 stages:

1. `Detecting hardware`_.

2. `Initializing essential structures`_ (buddy allocator, interrupt table ...).

3. `Run ``init`` process`_.


Detecting hardware
''''''''''''''''''

Hardware detection must be done first to figure out which memory is free
to use.

The standard mechanism uses *device trees*. The interpreter is written such
that it only uses stack space to iterate all entries. It is possible to write
it such that it uses only a fixed amount of memory but it hasn't been deemed
worth the effort.

The kernel currently only supports a single ``memory`` device. While systems
with multiple ``memory`` devices may exist, no such systems exist to the best
of my knowledge.


Initializing essential structures
'''''''''''''''''''''''''''''''''

When a memory device has been found, a *buddy allocator* is initialized.
Then the *interrupt / trap table* is initialized. With that, the essential
structures needed for running tasks are in place.


Run ``init`` process
''''''''''''''''''''

Finally, the ``init`` process is started. The ``init`` binary is an ELF
file included with the kernel


Page allocation
~~~~~~~~~~~~~~~

All physical pages are managed using a *buddy allocator*.

On boot and when deallocating memory, the page is cleared. This helps
prevent inadvertent information leaks and makes allocation faster.


Managing devices
~~~~~~~~~~~~~~~~

Devices are found by interpreting a *device tree*. Communication occurs with
MMIO and/or DMA. A task can reserve one or more devices, which is achieved by
the kernel mapping the MMIO/DMA area into the task's address space.


File I/O & IPC
~~~~~~~~~~~~~~

File I/O and IPC use the same mechanisms provided by ``io_ring``. When a task
begins running, it is put in an array with running tasks, where each index maps
to a unique hart.

When a task is halted or explicitly requests it, a hart will process all
entries in the request and respond queues and pass them on to the appropriate
tasks.

To prevent stalling task switching by processing an extreme amount of entries,
the queues are limited in size.


Kernel tracking
'''''''''''''''

To keep track of pending requests, the kernel keeps a request mapping in the
*responding* task.

The *requesting* tasks have a small mapping that keeps track of the amount
pending responses per task. This is so that if the requesting task dies, the
responses can be cancelled immediately.

It is possible that a task may receive more completion events than there are
entries in the completion queue. It is up to the task to ensure this doesn't
happen.


Traps (system calls & interrupts)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

All traps preserve all registers. This is mainly to prevent information leaks
but it also helps keep context switching simple.

All system calls use a predictable amount of time: they avoid any operations
that may take a long, variable amount of time such as I/O. The only exception
is ``io_wait``.
