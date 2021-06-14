========================
Third-party repositories
========================

While developing everything from scratch is definitely possible if you
have a couple decades of free time, it is often more practical to reuse
existing code. One big disadvantage is that it's harder to audit code
you don't actively maintain.

To alleviate the difficulties of auditing foreign code, all repositores
are cloned here and effectively "frozen". If code never changes, it doesn't
need to be audited over and over again. Of course, updates may be necessary
from time to time but this should be rare.


Why subtrees instead of submodules?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Git's subtrees were chosen over submodules as the latter rely on a host
remaining online. While the remote of a submodule is easy to change, it
still only moves the problem. With subtrees all the code is included
directly in the base repository and no fiddling with remotes is ever
needed.


Adding third-party repositories
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Third-party code will only be included if it fullfills any of the following
conditions:

* The code addresses a complex problem that would take (too) much effort to
  reproduce. For example, cryptography libraries are generally hard to reproduce
  as they also need to be watertight against exploits.

* The code addresses a common need that results in more than just a few lines
  in every project. For example, ``volatile`` is trivial but reimplementing
  the functionality over and over again is tedious. ``ìs-odd`` is pure stupidity
  however.

Additionaly, the code has to come from either a well-known (i.e. popular) source
or be trivial to audit.

Also remember to squash the history!
