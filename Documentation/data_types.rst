==========
Data types
==========

There are a few common data types:


Byte
~~~~

A byte refers to the smallest addressable type, which is always an ``u8``.


Integer
~~~~~~~

Integer types are prefixed with either a ``u``, which indicates an *unsigned*
number, or an ``s``, which indicates a *signed* number.

An integer is suffixed with a decimal number indicating the amount of bits
the number occupies (e.g. ``8`` means it occupies 8 bits, ``16`` occupies
16 bits ...). ``size`` is a special suffix that indicates an integer with
the same width as that of the databus (e.g. 32 bits on 32-bit architectures,
64 bits on 64 bit architectures.


Pointers
~~~~~~~~

Pointers are prefixed with either ``*mut`` or ``*const`` and suffixed with
any other datatype. A ``*mut`` pointer refers to data that can be mutated,
a ``*const`` pointer refers to data that cannot be mutated (but may still
change through another mutable pointer!).


Strings
~~~~~~~

Strings always refer to a sequence of valid UTF-8 characters. The sequence
itself is indicated with a ``str``, which does **not** include the length.

Strings are commonly stored as a ``small_str``, which is prefixed by an
``u8`` indicating the *total* length of the string (i.e. it indicates
the amount of bytes, *not* the amount of characters).


Memory pages
~~~~~~~~~~~~

A memory page is represented with ``mem_page`` and is always aligned to a
page boundary.
