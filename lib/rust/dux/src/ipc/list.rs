use core::convert::{TryFrom, TryInto};
use core::fmt;
use core::mem;
use core::slice;
use core::str;

/// A listing of an object's children.
#[repr(C)]
pub struct List<'a> {
	data: &'a [kernel::Page],
}

impl<'a> List<'a> {
	/// Wrap the given pages to interpret as a `List`.
	#[inline(always)]
	pub fn new(data: &'a [kernel::Page]) -> Self {
		Self { data }
	}

	/// Iterate over all the entries in this list.
	#[inline(always)]
	pub fn iter<'b>(&'b self) -> Iter<'b, 'a> {
		Iter {
			list: self,
			index: 0,
		}
	}

	/// Get a specific entry in the list.
	#[inline(always)]
	pub fn get(&self, index: usize) -> Option<Entry<'a>> {
		let entries = unsafe { self.data.as_ptr().cast::<usize>().add(1).cast::<RawEntry>() };
		let entries = unsafe { slice::from_raw_parts(entries, self.len()) };
		entries.get(index).map(|e| {
			let len = self.data.len() * mem::size_of::<kernel::Page>();
			let data = unsafe { slice::from_raw_parts(self.data.as_ptr().cast(), len) };
			let start = usize::try_from(e.name_offset).unwrap();
			Entry {
				uuid: e.uuid,
				name: start
					.checked_add(e.name_length.into())
					.and_then(|end| data.get(start..end)),
				size: e.size
			}
		})
	}

	/// Get the amount of entries in this list.
	#[inline(always)]
	pub fn len(&self) -> usize {
		if self.data.is_empty() {
			0
		} else {
			unsafe { *self.data.as_ptr().cast::<usize>() }
		}
	}
}

/// A single entry in an object list.
pub struct Entry<'a> {
	/// The UUID of the object.
	pub uuid: kernel::ipc::UUID,
	/// The name of the object, if any.
	///
	/// This will also be `None` if the name couldn't be fetched, i.e `RawEntry::name_length` or
	/// `RawEntry::name_offset` were out of range.
	pub name: Option<&'a [u8]>,
	/// The size of the object. Usually, this limit is expressed in bytes.
	pub size: u64,
}

impl fmt::Debug for Entry<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut d = f.debug_struct("Entry");
		d.field("uuid", &self.uuid);
		self.name.map(|name| {
			str::from_utf8(name)
				.map(|name| { d.field("name", &name); })
				.map_err(|name| { d.field("name", &name); });
		});
		d.field("size", &self.size);
		d.finish()
	}
}

/// A single raw entry in an object list.
#[repr(C)]
pub struct RawEntry {
	pub uuid: kernel::ipc::UUID,
	pub size: u64,
	pub name_offset: u32,
	pub name_length: u16,
}

pub struct Iter<'a, 'b> {
	list: &'a List<'b>,
	index: usize,
}

impl<'b> Iterator for Iter<'_, 'b> {
	type Item = Entry<'b>;

	fn next(&mut self) -> Option<Self::Item> {
		let e = self.list.get(self.index);
		e.is_some().then(|| self.index += 1);
		e
	}
}

/// A builder for creating `List´ structures. It allocates pages as needed.
pub struct Builder {
	address: crate::Page,
	page_count: usize,
	max_pages: usize,
	strings_offset: usize,
	index: usize,
}

#[derive(Debug)]
pub enum BuilderAddError {
	MemoryAllocationError,
	MaxPagesExceeded,
	NameTooLong,
}

impl Builder {
	/// Create a new builder. This does not allocate any pages but it does reserve some.
	///
	/// The `max_entries` and `max_string_len` are used to determine how many pages need
	/// to be reserved. These should be estimated higher than necessary if in doubt.
	#[inline(always)]
	pub fn new(
		max_entries: usize,
		max_string_len: usize,
	) -> Result<Self, crate::mem::ReserveError> {
		let strings_offset = mem::size_of::<usize>() + max_entries * mem::size_of::<RawEntry>();
		let max_size = strings_offset + max_string_len;
		let max_pages = crate::Page::min_pages_for_range(max_size);
		crate::mem::reserve_range(None, max_pages).map(|address| Self {
			address,
			page_count: 0,
			max_pages,
			strings_offset,
			index: 0,
		})
	}

	/// Get the slice the builder is operating on.
	#[inline(always)]
	pub fn data<'a>(&'a self) -> &'a [kernel::Page] {
		unsafe { slice::from_raw_parts(self.address.as_ptr(), self.page_count) }
	}

	/// Add an entry
	#[inline]
	pub fn add(&mut self, uuid: kernel::ipc::UUID, name: &[u8], size: u64) -> Result<(), BuilderAddError> {
		let name_length = name
			.len()
			.try_into()
			.map_err(|_| BuilderAddError::NameTooLong)?;
		let str_end = self.strings_offset + name.len();
		(str_end < self.max_data())
			.then(|| ())
			.ok_or(BuilderAddError::NameTooLong)?;

		unsafe {
			if self.page_count == 0 {
				// Just allocate everything, I can't be bothered.
				let addr = self.address.as_ptr();
				let ret = kernel::mem_alloc(addr, self.max_pages, kernel::PROT_READ_WRITE);
				self.page_count = self.max_pages;
			}

			let offt = self.strings_offset;
			for (w, r) in self.data_u8_mut()[offt..].iter_mut().zip(name) {
				*w = *r;
			}
			let name_offset = self.strings_offset.try_into().unwrap();

			self.address
				.as_ptr()
				.cast::<usize>()
				.add(1)
				.cast::<RawEntry>()
				.add(self.index)
				.write(RawEntry {
					uuid,
					size,
					name_offset,
					name_length,
				});
			self.strings_offset = str_end;

			*self.address.as_ptr().cast::<usize>() += 1;
		}

		self.index += 1;

		Ok(())
	}

	/// The maximum amount of data that can be written.
	fn max_data(&self) -> usize {
		kernel::Page::SIZE * self.max_pages
	}

	/// Return the data as bytes.
	fn data_u8_mut<'a>(&'a mut self) -> &'a mut [u8] {
		unsafe {
			slice::from_raw_parts_mut(
				self.address.as_ptr().cast(),
				self.page_count * kernel::Page::SIZE,
			)
		}
	}
}

impl Drop for Builder {
	fn drop(&mut self) {
		if self.page_count > 0 {
			let ret = unsafe { kernel::mem_dealloc(self.address.as_ptr(), self.page_count) };
			assert_eq!(ret.status, 0, "failed to free memory: {}", ret.value);
		}
		crate::mem::unreserve_range(self.address, self.max_pages)
			.expect("failed to unreserve range");
	}
}
