========================
Standard library wrapper
========================

This library implements the standard C runtime as well as some other common
functionality from POSIX.


POSIX compatibility
~~~~~~~~~~~~~~~~~~~

While this library implement some of the POSIX functions on top of the kernel's
system calls, It is not intended to fully implement the standard. It merely
makes it easier to port existing applications.

The implementation is based on ``glibc`` documentation. I'd reference the
"official" standard but they charge 900$ for an *open standard*\* so they
can go fuck themselves.

If someone happens to have a link to a *free* copy, as in you don't need to
sacrifice anything, please let me know.

\* Allegedly there is a free version but it requires me to create an account
   and give away a load of personal info so that is not an option.


Contributing
~~~~~~~~~~~~

If a port of an application needs a certain function that is missing, then
a patch implementing it would be very welcome.

Patches that implement functionality without a corresponding application
port are likely to be rejected as there is nothing to test against.


Recommended reading
~~~~~~~~~~~~~~~~~~~

* `GNU's libc manual`_

* `POSIX Library Functions`_

* `C POSIX library`_

.. _`GNU's libc manual`: https://www.gnu.org/software/libc/manual/html_node/index.html
.. _`POSIX Library Functions`: https://www.mkompf.com/cplus/posixlist.html
.. _`C POSIX library`: https://en.wikipedia.org/wiki/C_POSIX_library
