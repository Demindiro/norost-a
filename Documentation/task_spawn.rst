==============
Spawning tasks
==============

This document describes common conventions when spawning tasks.


stdin/-out/-err ...
~~~~~~~~~~~~~~~~~~~

As there is no notion of a "file descriptor" at the lowest level, the program
must be told somehow where to read & write data.

This is done by pushing address + UUID entries on the stack. The amount of
entries is determined by an ``usize`` that is pushed first.

The program is free to interpret the given entries in any way, i.e. it is not
obliged to follow any standard such as POSIX.


POSIX compatibility
'''''''''''''''''''

For programs that assume a POSIX-y environment, the first entry is interpreted
as ``stdin`` & the second as ``stdout``. If a third entry is pushed it will be
used for ``stderr``, otherwise it will alias ``stdout``.


Passing arguments
~~~~~~~~~~~~~~~~~

After the address + UUID entries are parsed, the arguments are passed. Each
argument is a pointer to a string. Each string is prefixed by a ``u16``
indicating the length of the string. A pointer to each string is pushed onto
the stack. The amount of arguments is indicated by an ``usize`` that is pushed
before the string pointers.


Note on task 0
~~~~~~~~~~~~~~

The above implies that every spawned task has an initial stack and does not
need to create another one. However, this does not apply to the initial task
spawned by the kernel. It must provide a stack by itself and set up
stdin/-out/... in some other way.
