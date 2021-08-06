================
Interface design
================

This document describes best practices for implementing OS-wide interfaces.


Kernel vs Operating System
~~~~~~~~~~~~~~~~~~~~~~~~~~

While kernels and OSes are often lumped together, there is a clear difference.
This difference is doubly important in the context of microkernels.

Generally, the kernel should only provide the bare minimum of required
interfaces. This helps keeps the size down and improves overall efficiency.

For example, the kernel has no concept of a file system; it only facilitates
communication between tasks / processes. As a result, there is no kernel-
mandated format for directory listings. Instead, the format is defined by
the *OS*.

To emphasize the separation, there are separate *kernel* and *OS* libraries.
The kernel library defines the system calls and a few structures pertaining
to said calls. The OS library builds upon the kernel library and defines
additional interfaces to facilitate application development.


Struct optimization
~~~~~~~~~~~~~~~~~~~

To ensure multiple programming languages can interoperate, the *de facto*
"C ABI" is used. Usually, this means there may be padding inside ``struct`` s
to align the fields. To ensure as little space as possible is wasted, all
fields should be ordered by *alignment*. Generally the alignment is equal
to the size of the field.


Sharing data
~~~~~~~~~~~~

To avoid excessive copying, data is shared between processes by directly
mapping pages. Still, sharing pages isn't entirely free. To avoid excessive
sharing, two methods can be used:


Address objects with UUIDs
``````````````````````````

UUIDs are 128-bit large identifiers which, if properly generated, will always
be unique per process (and likely between everything on this planet too). UUIDs
can be sent directly via an IPC packet, avoiding any potential page sharing.

When the UUID of an object isn't known, a process can send an arbitrary
identifier via the packet's ``data`` field. The receiving process should
then return a non-zero UUID that the sending process can cache and reuse.


Share once and communicate via the mappings
```````````````````````````````````````````

If frequent communication between two processes is expected it may be best
to set up an alternative channel for communication via the shared pages.
The advantage is that this does not involve the kernel and doesn't require
context switches.
