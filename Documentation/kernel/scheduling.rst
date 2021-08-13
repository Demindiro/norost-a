==========
Scheduling
==========

One of the hardest problems is giving each task a fair amount of CPU time:
accounting for new vs old tasks, real time vs non-RT, priority ... is no easy
task.

While the current algorithm is by no means perfect, it should give every task
and task group a reasonable amount of CPU time while preventing malicious
applications from taking it all.


Algorithm
~~~~~~~~~

Task-level
''''''''''

To ensure every task and task group gets a fair share of CPU time, *fractional
scheduling* is used: every time a task stops, be it due to an interrupt or
it yielding, the amount of time used from it's slice is measured and added to
an accumulator. Before addition, the accumulator is reduced by a constant
factor; the formula is ``accumulator / factor_yield + fraction``.

To ensure that tasks that have a high accumulator value still can have it
reduced without being scheduled, the effective accumulated value is
``min(accumulator + (last_sched - now) * factor_time, 0)``.

Since some tasks may be more important than others, an accumulator factor
can be specified, which makes the effective accumulator value ``e_acc *
factor_acc``.


Group-level
'''''''''''

TODO


Real-time scheduling
~~~~~~~~~~~~~~~~~~~~

Some tasks need to be able to respond immediately to avoid lag, e.g. audio
servers need to process & forward requests as fast as possible.

A task can be marked as real-time by setting it's accumulator threshold. A task
whose accumulator is below the threshold can be scheduled immediately. If the
task exceeds its threshold it loses it real-time privileges however.

The task's threshold is multiplied with the group's threshold. This effectively
prevents any tasks in groups without real-time privileges from ever gaining
said privileges.


Queue
~~~~~

Each group has a separate heap where each entry is a task ID. The heap is
sorted based on the task's accumulator.

Groups are sorted in the same way.


Suspended tasks
~~~~~~~~~~~~~~~

Tasks may be waiting for an event to happen. There events are:

* Receiving an IPC packet.

* Receiving a notification.

* Waiting for a certain amount of time.

All of these are set using the ``io_wait`` call. The call takes a single
argument specifying how long to wait in microseconds.

When the task is rescheduled, the wait time is set to ``0``, i.e. the timeout
is cleared.
