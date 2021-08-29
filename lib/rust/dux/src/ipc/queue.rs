use crate::util::SpinLock;
use core::cell::{Cell, UnsafeCell};
use core::mem;
use core::ops;
use core::pin::Pin;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use kernel::ipc::{FreeRange, Packet};

/// The IPC structure shared with the kernel
#[repr(C)]
pub struct KernelIPC<const S: u16>
where
	[(); S as usize]: Sized,
{
	table: [UnsafeCell<Packet>; S as usize],
	free_ranges_list: [FreeRange; S as usize],
	transmit_queue_index: AtomicU32,
	received_queue_index: AtomicU32,
	free_packets_queue_index: AtomicU32,
	free_ranges_list_size: AtomicU32,
	transmit_queue: [Cell<u16>; S as usize],
	received_queue: [Cell<u16>; S as usize],
	free_packets_queue: [Cell<u16>; S as usize],
}

pub struct IPC<const S: u16>
where
	[(); S as usize]: Sized,
{
	kernel: KernelIPC<S>,
	last_received_index: Cell<u16>,
	transmit_lock: SpinLock,
	received_lock: SpinLock,
	free_packets_queue_lock: SpinLock,
	free_transmit_stack_lock: SpinLock,
	free_transmit_stack: [Cell<u16>; S as usize],
	free_transmit_stack_index: Cell<u16>,
}

impl<const S: u16> IPC<S>
where
	[(); S as usize]: Sized,
{
	/// Create a new IPC structure
	pub const fn new() -> Self {
		if S.count_ones() != 1 {
			panic!("size is not a power of two");
		}
		const ZEROED_PACKET: UnsafeCell<Packet> = UnsafeCell::new(Packet::ZEROED);
		const ZEROED_CELL: Cell<u16> = Cell::new(0);
		const ZEROED_RANGE: FreeRange = FreeRange {
			address: AtomicPtr::new(core::ptr::null_mut()),
			count: AtomicUsize::new(0),
		};
		Self {
			kernel: KernelIPC {
				table: [ZEROED_PACKET; S as usize],
				free_ranges_list: [ZEROED_RANGE; S as usize],
				transmit_queue_index: AtomicU32::new(0),
				received_queue_index: AtomicU32::new(0),
				free_packets_queue_index: AtomicU32::new(0),
				free_ranges_list_size: AtomicU32::new(0),
				transmit_queue: [ZEROED_CELL; S as usize],
				received_queue: [ZEROED_CELL; S as usize],
				free_packets_queue: [ZEROED_CELL; S as usize],
			},
			last_received_index: Cell::new(0),
			transmit_lock: SpinLock::new(),
			received_lock: SpinLock::new(),
			free_packets_queue_lock: SpinLock::new(),
			free_transmit_stack_lock: SpinLock::new(),
			free_transmit_stack: [ZEROED_CELL; S as usize],
			free_transmit_stack_index: Cell::new(0),
		}
	}

	pub const fn mask(&self) -> u16 {
		S - 1
	}

	/// Register this queue as the active queue to the kernel.
	pub fn activate(slf: Pin<&Self>) -> Result<(), usize> {
		let slf_ptr = &*slf as *const _ as *mut _;
		let ret = unsafe { kernel::io_set_queues(slf_ptr, slf.mask().count_ones() as u8) };
		(ret.status == 0).then(|| ()).ok_or(ret.status)
	}

	/// Add a free range
	pub fn add_free_range(&self, address: crate::Page, count: usize) -> Result<(), Full> {
		for e in self.kernel.free_ranges_list.iter() {
			if e.count.load(Ordering::Relaxed) == 0 {
				// The address must be set first. While the kernel may see this address
				// before the count is updated, it is no matter as the count is already
				// 0 and hence the kernel will skip it.
				e.address.store(address.as_ptr(), Ordering::Relaxed);
				e.count.store(count, Ordering::Relaxed);
				return Ok(());
			}
		}
		Err(Full)
	}

	/// Send an IPC packet to a task.
	///
	/// This will yield the task if no slots are available.
	pub fn transmit(&self, mut wait: impl FnMut()) -> TransmitLock<S> {
		mem::forget(self.transmit_lock.lock());
		loop {
			match self.free_stack_pop() {
				Ok(slot) => return TransmitLock { ipc: self, slot },
				Err(NoFreeSlots) => wait(),
			}
		}
	}

	/// Attempt to reserve a slot for sendng an IPC packet to a task.
	pub fn try_transmit(&self) -> Result<TransmitLock<S>, NoFreeSlots> {
		let lock = self.transmit_lock.lock();
		let slot = self.free_stack_pop()?;
		mem::forget(lock);
		Ok(TransmitLock { ipc: self, slot })
	}

	/// Receive an IPC packet.
	///
	/// This will yield the task if no packets have been received yet.
	pub fn receive(&self) -> Received<S> {
		let lock = self.received_lock.lock();
		loop {
			let i = self.last_received_index.get();
			if i != self.kernel.received_queue_index.load(Ordering::Acquire) as u16 {
				return Received {
					ipc: &self,
					slot: self.kernel.received_queue[usize::from(i & self.mask())].get(),
				};
			}
			unsafe { kernel::io_wait(u64::MAX) };
		}
	}

	/// Attempt to reserve a slot for sendng an IPC packet to a task.
	pub fn try_receive(&self) -> Option<Received<S>> {
		let lock = self.received_lock.lock();
		let i = self.last_received_index.get();
		(i != self.kernel.received_queue_index.load(Ordering::Acquire) as u16).then(|| Received {
			ipc: &self,
			slot: self.kernel.received_queue[usize::from(i & self.mask())].get(),
		})
	}

	/// Add an unused slot to the free queue for the kernel to use.
	fn free_received_queue_push(&self, slot: u16) -> Result<(), Full> {
		self.free_packets_queue_lock.lock();
		let i = self.kernel.free_packets_queue_index.load(Ordering::Relaxed);
		((i as u16) < S).then(|| ()).ok_or(Full)?;
		self.kernel.free_packets_queue[usize::from(i as u16 & self.mask())].set(slot);
		self.kernel
			.free_packets_queue_index
			.store(i + 1, Ordering::Release);
		Ok(())
	}

	fn free_stack_pop(&self) -> Result<u16, NoFreeSlots> {
		self.free_transmit_stack_lock.lock();
		let i = self
			.free_transmit_stack_index
			.get()
			.checked_sub(1)
			.ok_or(NoFreeSlots)?;
		self.free_transmit_stack_index.set(i);
		Ok(self.free_transmit_stack[usize::from(i)].get())
	}

	/// Add an unused slot to the free stack.
	fn free_stack_push(&self, slot: u16) -> Result<(), Full> {
		self.free_transmit_stack_lock.lock();
		let i = self.free_transmit_stack_index.get();
		(i < S).then(|| ()).ok_or(Full)?;
		self.free_transmit_stack[usize::from(i)].set(i);
		self.free_transmit_stack_index.set(i + 1);
		Ok(())
	}
}

#[derive(Debug)]
pub struct Full;

#[derive(Debug)]
pub struct NoFreeSlots;

/// A lock on the transmit queue along with the slot of the packet to write to.
pub struct TransmitLock<'a, const S: u16>
where
	[(); S as usize]: Sized,
{
	ipc: &'a IPC<S>,
	slot: u16,
}

impl<'a, const S: u16> TransmitLock<'a, S>
where
	[(); S as usize]: Sized,
{
	/// Cancel the transmission and unlock the queue.
	pub fn cancel(self) {
		unsafe { self.ipc.transmit_lock.unlock() };
		mem::forget(self);
	}

	pub fn into_raw(self) -> (&'a IPC<S>, u16, &'a mut kernel::ipc::Packet) {
		let (ipc, slot) = (self.ipc, self.slot);
		mem::forget(self);
		(ipc, slot, unsafe {
			&mut *ipc.kernel.table[usize::from(slot)].get()
		})
	}

	pub unsafe fn from_raw(ipc: &'a IPC<S>, slot: u16) -> Self {
		Self { ipc, slot }
	}
}

impl<const S: u16> ops::Deref for TransmitLock<'_, S>
where
	[(); S as usize]: Sized,
{
	type Target = kernel::ipc::Packet;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.ipc.kernel.table[usize::from(self.slot)].get() }
	}
}

impl<const S: u16> ops::DerefMut for TransmitLock<'_, S>
where
	[(); S as usize]: Sized,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.ipc.kernel.table[usize::from(self.slot)].get() }
	}
}

impl<const S: u16> Drop for TransmitLock<'_, S>
where
	[(); S as usize]: Sized,
{
	fn drop(&mut self) {
		let index = &self.ipc.kernel.transmit_queue_index;
		let queue = &self.ipc.kernel.transmit_queue;

		let i = index.load(Ordering::Relaxed);
		queue[usize::from(i as u16 & self.ipc.mask())].set(self.slot);
		index.store(i.wrapping_add(1), Ordering::Release);

		unsafe { self.ipc.transmit_lock.unlock() };
	}
}

/// A wrapper around a slot of a received packet.
///
/// This does not lock the receive queue. On drop, it will push the slot to the free queue however.
pub struct Received<'a, const S: u16>
where
	[(); S as usize]: Sized,
{
	ipc: &'a IPC<{ S }>,
	slot: u16,
}

impl<'a, const S: u16> Received<'a, S>
where
	[(); S as usize]: Sized,
{
	pub fn into_raw(self) -> (u16, &'a mut kernel::ipc::Packet) {
		let (slot, ipc) = (self.slot, self.ipc);
		mem::forget(self);
		(slot, unsafe {
			&mut *ipc.kernel.table[usize::from(slot)].get()
		})
	}

	pub unsafe fn from_raw(ipc: &'a IPC<{ S }>, slot: u16) -> Self {
		Self { slot, ipc }
	}

	/// Release the lock but don't discard the packet. Instead, swap the packet with the last
	/// available entry in the ring buffer.
	pub fn defer(self) {
		todo!()
		/*
		self.ipc.received_lock.lock();
		let index = &self.ipc.kernel.received_queue_index;
		let queue = &self.ipc.kernel.received_queue;
		let last_index = self.ipc.last_received_index.get();
		let mask = self.ipc.mask();

		let prev_index = index.wrapping_sub(1);
		let a = entries[usize::from(last_index & mask)].get();
		let b = entries[usize::from(prev_index & mask)].get();
		debug_assert_eq!(a, self.slot, "current received entry mutated while locked");
		entries[usize::from(prev_index & mask)].set(a);
		entries[usize::from(last_index & mask)].set(b);

		// Prevent the packet slot from being added to the free queue
		mem::forget(self);
		*/
	}

	/// Reserve the slot for transmission, not pushing it to the free transmit list.
	pub fn reserve_transmission(self) {
		todo!();
	}
}

impl<const S: u16> ops::Deref for Received<'_, S>
where
	[(); S as usize]: Sized,
{
	type Target = kernel::ipc::Packet;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.ipc.kernel.table[usize::from(self.slot)].get() }
	}
}

impl<const S: u16> ops::DerefMut for Received<'_, S>
where
	[(); S as usize]: Sized,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.ipc.kernel.table[usize::from(self.slot)].get() }
	}
}

impl<const S: u16> Drop for Received<'_, S>
where
	[(); S as usize]: Sized,
{
	fn drop(&mut self) {
		self.ipc.free_packets_queue_lock.lock();

		let index = &self.ipc.kernel.free_packets_queue_index;
		let queue = &self.ipc.kernel.free_packets_queue;

		let i = index.load(Ordering::Relaxed);
		queue[usize::from(i as u16 & self.ipc.mask())].set(self.slot);
		index.store(i.wrapping_add(1), Ordering::Release);
	}
}
