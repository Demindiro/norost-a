//! Global kernel VMS


pub fn add_kernel_mapping<F: FnMut() -> super::PPN>(f: F, count: usize, rwx: crate::arch::RWX) -> core::ptr::NonNull<crate::arch::Page> {
	crate::arch::add_kernel_mapping(f, count, rwx)
}
