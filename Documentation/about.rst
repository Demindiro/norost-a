=====
About
=====


Goals
~~~~~~

* Distributed OS.

* Filesystem access on any machine.

* Inter-process communication between any machines.

* Asychronous whenever possible with optional explicit synchronization.

* (Mostly) architecture-independent.

  * There is no intent to support machines which don't have 8-bit bytes, nor
    will extremely resource-constrained (e.g. < 1kB memory) be considered.

* Migrateable processes.

  * Processes can be transferred to different machines transparently. This
    would allow load-balancing without any manual involvement or implementing
    support in the application itself.

* Handle sudden loss of remote resources gracefully.

  * This is a requirement for a stable distributed OS. The OS must keep
    running even if any machine fails.

* Secure authentication & communication

  * To prevent malicious actors from accessing the OS, authentication and
    encrypting communications is necessary.

* Flat model

  * No process trees. "Parenting" is handled by watching processes.

  * No mounting of filesystems on directories.

  * This should scale better since you don't have arbitrarily deep trees.
