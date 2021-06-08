//! Simple stack-based memory allocator.
//!
//! It keeps track of all free physical pages by putting them in a big array,
//! i.e. stack. When allocating, it pops the last address. When deallocating,
//! it pushes the address.


