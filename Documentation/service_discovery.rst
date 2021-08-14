=================
Service Discovery
=================

All operating systems need some standards on how to discover certain services;
e.g. on Linux devices go in ``/dev``, X11 uses port 6000 ...

This document describes the common way to discover services in Dux.


Kernel registry
~~~~~~~~~~~~~~~

Services can be registered & discovered through the kernel's *registry*: the
registry is a list of names mapping to a task address.

For security purposes, only tasks in group 0 can reserve registry entries. This
is to avoid hijacking names that could confuse the user or other tasks.

To allow tasks in others group to reserve a registry entry, tasks in group 0
can register entries on other tasks' behalf by passing a non- ``usize::MAX``
argument as the address parameter.

While the registry could be implemented as a task, it is considered critical
enough that it is included in the kernel itself.


Entry format
''''''''''''

Entries consist of merely two parts: a name and an address.

Names are byte strings of up to 255 characters long. While the strings do not
have to be valid UTF-8, it is highly recommended to make them so.

255 characters are deemed plenty long enough to allow human-readable unique
identifiers.
