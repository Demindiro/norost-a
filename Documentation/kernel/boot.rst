============
Boot process
============

This document describes the steps taken by the kernel to go from early boot
to a functional userspace.

The steps are:

1) Set up the page tables and enable virtual memory system.

2) Set up stack space and the trap handler.

3) Parse the device tree, ACPI tables or whatever relevant structure for the
   platform. From the structure, find some unused memory and insert it in the
   physical memory manager. The PMM also initializes itself and the virtual
   memory manager at this point.

4) Start the init process.


Setting up page tables and enabling VMS
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The page tables are set up by a small assembly script. This script is
position-independent and includes the kernel binary in its data section.

The kernel is included as an ELF file. THe script uses the information in
the ELF file to map the kernel to the higher half of memory. It then identity
maps the lower half of memory. Finally it enables the VMS and jumps to the
kernel's entry point.

Normally, this script is a raw binary but it should also be useable as
an ELF file depending on the bootloader.


Setting up the stack and trap handler
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The kernel immediately sets up the stack, then enters ``main``. In ``main``,
the trap handler is set up. The handler can detect if anything goes wrong
during early boot and halts the system if it does so. After boot, it will
handle traps normally.


Reserving memory
~~~~~~~~~~~~~~~~

As the kernel needs some memory to at least start the init process, it parses
the platform's info structure to find a region of memory and inserts it into
the PMM. The PMM uses some of this memory to set up the tracking structures
of itself and the VMM.

    Later kernels should have a .bss section instead which is allocated by the
    bootloader. Any additional memory can then be inserted by the init process
    instead. Current init programs should be written with the assumption that
    there may only be a small amount of memory available early on.


Starting the init process
~~~~~~~~~~~~~~~~~~~~~~~~~

Finally, the init process is started. This task will parse the platform's info
structure and load appropriate drivers as needed.

Parsing of the structure is handled by the init process as it can be very
complex and while there are standards for hardware, there is still a lot of
variation. Supporting it in the kernel is possible, but it may cause the kernel
to become bloated.

The tradeoff is that the init process has (needs) nigh omnipotence over the
system's resources. To reduce the risk of bugs in root processes exposing
critical vulnerabilities, tasks have a "driver" flag. This flag indicates
whether the kernel should accept or reject requests for direct access to
system resources. The flag can be toggled on and off at any time.
