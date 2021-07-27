//! # Task groups
//!
//! To fairly distribute CPU & memory resources, task groups are used. The
//! resources in a single task group are accessible to all processes in that
//! group. Resources can further be reserved per task.

use super::Task;
use crate::allocator::arena;
use crate::memory::reserved;
use core::ops::Deref;
use core::ptr;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicPtr, Ordering};

/// The start of the task group list.
static GROUPS: arena::Arena<GroupData> = unsafe {
	arena::Arena::new(
		reserved::TASK_GROUPS.start.cast(),
		reserved::TASK_GROUPS.byte_count(),
	)
};

/// A group of tasks
pub struct GroupData {
	/// A list of tasks. This is currently hardcoded to 4 because I'm lazy.
	tasks: [AtomicPtr<super::TaskData>; 4],
}

// FIXME Task is not sync yet. We also need to ensure tasks can't be removed/freed while referenced.
unsafe impl Sync for GroupData {}

#[derive(Debug)]
pub struct NoTask;

#[derive(Debug)]
pub struct Full;

/// A safe wrapper around `GroupData` that can be manually freed but also includes
/// checks to prevent use-after-frees.
///
/// This structure has interior mutability, i.e. anything with an "immutable" reference
/// may still change some of the data it points to.
pub struct Group<'a> {
	data: arena::Guard<'a, GroupData>,
	index: usize,
}

impl Group<'_> {
	/// Create a new task group & insert the given task.
	///
	/// Returns the group ID.
	// TODO avoid using NonNull
	pub fn new(task: Task) -> Result<usize, arena::InsertError> {
		GROUPS.insert(GroupData {
			tasks: [
				AtomicPtr::new(task.0.as_ptr()),
				AtomicPtr::default(),
				AtomicPtr::default(),
				AtomicPtr::default(),
			],
		})
	}

	/// Get a reference to a task in this group
	pub fn task(&self, id: usize) -> Result<Task, NoTask> {
		let tasks = unsafe { &self.data.tasks };
		tasks
			.get(id)
			.and_then(|ptr| unsafe { ptr.load(Ordering::Relaxed).as_ref() })
			.map(|t| Task(NonNull::from(t)))
			.ok_or(NoTask)
	}

	/// Remove a task. This frees the group if no task are left.
	///
	/// If any tasks are left, the group itself is returned.
	// FIXME this isn't thread-safe
	pub fn remove_task(mut self, id: usize) -> Result<Option<Self>, NoTask> {
		let tasks = unsafe { &self.data.tasks };
		tasks
			.get(id)
			.map(|t| t.store(ptr::null_mut(), Ordering::Relaxed));
		// FIXME this is not sound
		if tasks
			.iter()
			.all(|ptr| ptr.load(Ordering::Relaxed).is_null())
		{
			unsafe {
				GROUPS.remove(self.index);
			}
			Ok(None)
		} else {
			Ok(Some(self))
		}
	}

	/// Returns the ID of this group.
	pub fn id(&self) -> usize {
		self.index
	}

	/// Insert a new task.
	pub fn insert(&self, task: Task) -> Result<usize, Full> {
		for (i, s) in self.data.tasks.iter().enumerate() {
			if s.load(Ordering::Relaxed).is_null() {
				if s.compare_exchange(
					ptr::null_mut(),
					task.0.as_ptr(),
					Ordering::Relaxed,
					Ordering::Relaxed,
				)
				.is_ok()
				{
					return Ok(i);
				}
			}
		}
		Err(Full)
	}
}

impl Group<'static> {
	/// Get the group with the given ID.
	pub fn get(id: usize) -> Option<Self> {
		let index = id;
		GROUPS.get(id).map(|data| Self { data, index })
	}
}

/// A guard around a task structure
pub struct Guard<'a> {
	_marker: core::marker::PhantomData<&'a ()>,
}

impl Deref for Guard<'_> {
	type Target = Task;

	fn deref(&self) -> &Self::Target {
		todo!()
	}
}
