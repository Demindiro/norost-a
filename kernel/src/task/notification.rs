//! # Notification handlers & helpers
//!
//! While notifications are a form of IPC, they also behave significantly differently from the
//! "regular" IPC, hence why notifications are treated as a separate thing.

use core::mem;
use core::ptr::{self, NonNull};
use core::sync::atomic::Ordering;

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
	Busy,
}

impl super::Task {
	/// Set the notification handler of this task, returning the previous
	/// one, if any.
	pub fn set_notification_handler(&self, handler: Option<Handler>) -> Option<Handler> {
		let handler = handler
			.as_ref()
			.map(Handler::as_ptr)
			.unwrap_or_else(ptr::null);
		let handler = self
			.notification_handler
			.swap(handler as *mut _, Ordering::Relaxed);
		NonNull::new(handler).map(|c| c.cast()).map(Handler)
	}
}
