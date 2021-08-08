use core::convert::TryInto;
use core::mem;

#[cfg(not(any(target_pointer_width = "16", target_pointer_width = "32", target_pointer_width = "64", target_pointer_width = "128")))]
compile_error!("Please report your alien computer to the local authorities");

/// A task ID, which is unique per task group.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
#[cfg(target_pointer_width = "16")]
pub struct TaskID(u8);
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
#[cfg(target_pointer_width = "32")]
pub struct TaskID(u16);
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
#[cfg(target_pointer_width = "64")]
pub struct TaskID(u32);
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
#[cfg(target_pointer_width = "128")]
pub struct TaskID(u64);

/// A task group ID, which is unique per system (i.e. it is unique globally in the kernel).
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
#[cfg(target_pointer_width = "16")]
pub struct GroupID(u8);
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
#[cfg(target_pointer_width = "32")]
pub struct GroupID(u16);
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
#[cfg(target_pointer_width = "64")]
pub struct GroupID(u32);
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
#[cfg(target_pointer_width = "128")]
pub struct GroupID(u64);

/// An address, which is composed of a task group ID and a group-local task ID
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(transparent)]
pub struct Address(usize);

impl Address {
	pub fn new(task: TaskID, group: GroupID) -> Self {
		Self(task.0 as usize | (group.0 << (mem::size_of::<usize>() * 4)) as usize)
	}

	pub fn task(&self) -> TaskID {
		TaskID((self.0 & ((0x100 << mem::size_of::<TaskID>()) - 1)).try_into().unwrap())
	}

	pub fn group(&self) -> GroupID {
		GroupID((self.0 >> (mem::size_of::<usize>() * 4)).try_into().unwrap())
	}

	pub(super) const fn todo(n: usize) -> Self {
		Self(n)
	}
}

impl From<TaskID> for usize {
	fn from(id: TaskID) -> Self {
		id.0 as usize
	}
}

impl From<GroupID> for usize {
	fn from(id: GroupID) -> Self {
		id.0 as usize
	}
}
