===============================
Dux resource management library
===============================

This library ensures that various low-level libraries can work nicely together.
It keeps track of memory mappings and the current I/O client & server buffers.

While there is (will be) an API to get this information from the kernel, syscalls
are slow and mapping memory directly may still surprise libraries / applications.

Features
~~~~~~~~

* Keeps track of memory mappings & allows reserving ranges without actually
  allocating any pages.

* Keeps track of I/O buffers & allows resizing / relocating them.
