//! # Notification handlers & helpers
//!
//! While notifications are a form of IPC, they also behave significantly differently from the
//! "regular" IPC, hence why notifications are treated as a separate thing.

use core::mem;
use core::ptr::NonNull;

/// A pointer to a userspace notification handler
pub struct Handler(NonNull<()>);

#[derive(Debug)]
pub enum NewHandlerError {}

impl Handler {
	/// Create a new handler.
	pub fn new(function: NonNull<()>) -> Result<Self, NewHandlerError> {
		Ok(Self(function))
	}

	/// Call the handler. This causes a context switch.
	#[inline(always)]
	fn call(&self) {
		todo!();
	}

	/// Return the inner pointer.
	pub fn as_ptr(&self) -> *const () {
		self.0.as_ptr()
	}
}

#[derive(Debug)]
pub enum SendError {
	/// The task did not specify a handler and hence cannot receive any notifications.
	NoHandler,
	/// The task is already processing a notification.
	#[allow(dead_code)]
	Busy,
}

impl super::Task {
	/// Send a notification to this task.
	#[allow(dead_code)]
	pub fn send_notification(&self) -> Result<(), SendError> {
		self.inner()
			.notification_handler
			.as_ref()
			.map(|handler| {
				handler.call();
			})
			.ok_or(SendError::NoHandler)
	}

	/// Set the notification handler of this task, returning the previous
	/// one, if any.
	pub fn set_notification_handler(&self, handler: Option<Handler>) -> Option<Handler> {
		mem::replace(&mut self.inner().notification_handler, handler)
	}
}
