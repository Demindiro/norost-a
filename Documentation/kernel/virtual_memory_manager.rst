======================
Virtual memory manager
======================


Since creating a good VMM model is *fucking* hard I decided to document how
exactly it works (in fact, the documentation is written *before* the
implementation even exists).


Goals
~~~~~

First, we should address what things we need from our VMM:

* We need to be able to share physical pages with other tasks.

* We need to be able to update the page tables relatively quickly.

* We need to have global kernel mappings and local user mappings. Ideally
  local kernel mappings are also supported.

* Large contiguous mappings with holes.

The first requirement means that ideally, we can directly modify the tables
of other tasks. A queue could also be used in which case pages are only mapped
in whenever the corresponding tables are loaded.

The second requirement means we should avoid flushing too many entries from
the TLB while still not implementing an overly complicated algorithm that is
so expensive any TLB performance hits will look small in comparison.


Memory reservations
~~~~~~~~~~~~~~~~~~~

To support **large contiguous mappings** address ranges can be reserved at
compile time. A reservation may look like this::

  VMM:
    VMM_PPN2        0xfffffffffffff000-0xffffffffffffffff
    VMM_PPN1        0xffffffffffe00000-0xffffffffffffefff
    VMM_PPN0        0xffffffffc0000000-0xffffffffffdfffff
    VMM             0xffffffffc0000000-0xffffffffffffffff
  Global:
    KERNEL          0xffffffffbfff0000-0xffffffffbfffffff
    PMM_BITMAP      0xffffffff9fff0000-0xffffffffbffeffff
    PMM_STACK       0xffffffff9efec000-0xffffffff9ffeffff
    SHARED_COUNTERS 0xfffffffb9efec000-0xffffffff9efebfff
    SHARED_ALLOC    0xfffffffb9edec000-0xfffffffb9efebfff
    GLOBAL          0xfffffffb80000000-0xffffffffbfffffff
  Local:
    SCRATCH_A       0xfffffffb7ffff000-0xfffffffb7fffffff
    SCRATCH_B       0xfffffffb7fffe000-0xfffffffb7fffefff
    SCRATCH_C       0xfffffffb7fffd000-0xfffffffb7fffdfff
    LOCAL           0xfffffffb40000000-0xfffffffb7fffffff

An additional advantage is that memory addresses are known beforehand, which
greatly simplifies other code.


"Hugepage windows" AKA ``HIGHMEM``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

To be able to access physical memory "directly" an approach commonly known
as ``HIGHMEM`` is used. See this LWN_ article for some details.

Let's define the basic set of operations we need to be able to do:

* Map in a page that needs to be operated on.

* Unmap the page we're operating on.

* Flush the page we (un)mapped in the TLB.

To prevent needing a lock, each hart gets a separate page table to operate on.
This also avoids the need for a TLB shootdown.

Next, let's consider the impact of setting & flushing an entry:

* We write the entry, which will cause the table to be loaded into the cache.

* We access the memory, which causes the TLB to do a lookup. Since the table
  (and the entry!) is already in cache this should be reasonably fast (about
  ~7 cycles).

* We clear the entry and flush the address in the TLB, both of which are
  fast.

You could avoid a TLB flush by first checking the current value of the entry
(and not flushing it immediately after) but since that will involve a load,
which will cause the table to be cached, and an extra branch it is likely
more expensive anyways.

.. _LWN: https://lwn.net/Articles/356378/


Modifying page tables
~~~~~~~~~~~~~~~~~~~~~

Now comes the *really fucking hard* part: modifying the page tables.

The big issue with page tables is that they use *physical page numbers*.
However, we operate with *virtual addresses*, so to modify the pages we
need to map them in... Mind blown.

There are three common / "obvious" methods to handle this situation:

* Recursive mapping. Basically, you map the root page table to the last
  address which then let us modify the tables thanks to the peculiarities
  in how it works. However, this doesn't work on all architectures (notably,
  it works on x86n but not on RISC-V). It also doesn't allow us to modify
  other tables directly.

* Identity mapping. If memory is identity mapped you can just treat the table
  as you would in a regular userspace program (i.e. with direct "pointers").
  It's easy, but requires switching address spaces and perhaps a TLB flush.

* "Hugepage windows" AKA ``HIGHMEM``. A few entries in the (root) pagetable are
  reserved for mapping specific pages. When done, the pages are unmapped and
  the corresponding addresses flushed. This does not require a TLB flush.
  It does require mapping the page table but mapping a single page isn't too
  bad.

* Map everything in kernel space. Good luck modifying the kernel map itself.

The recursive method is unusable because it doesn't work on architectures like
RISC-V (unless you're willing to do things like "duplicating"/aliasing pages,
which is horribly complicated. Trust me).

The second method is useable but if the architecture doesn't support address
tags it will have horrible performance because of potential TLB flushes.

The third method isn't great either as it may require frequent mapping and
unmapping but doesn't require changing address spaces. Besides, since the
table will be in cache (for writing) TLB fetches should still be fast.

Since it's likely (i.e. it would very much surprise me if not) that the
majority of new architectures / CPUs will support address tags in some form
the tradeoff is considered acceptable. Older systems suffer, but there is
not much that can be done about that short of designing a separate kernel
for them.
