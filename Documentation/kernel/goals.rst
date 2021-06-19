=====
Goals
=====

As a microkernel there are only a few goals that need to be met to allow a
functional and useful system:

* The kernel must schedule multiple tasks in a fair way.

* The kernel must be able to provide (memory) resources to tasks.

* The kernel must facilitate communication between tasks.

* The kernel must isolate tasks, i.e. it must enforce permissions for security.

Any other goals can be achieved by tasks themselves.
