//! Drivers for filesystems

mod cpio;
pub mod initramfs;

/// Trait implemented by all filesystem drivers
///
/// It defines a number of methods to load data from and store data in a backing store.
pub trait FileSystem {
	/// Returns information about the file with the given path
	fn info(&self, path: &str) -> Result<FileInfo, InfoError>;

	/// Reads data from the file given by the path and writes it into the given buffers.
	fn read(&self, path: &str, buffers: &mut [u8]) -> Result<(), ReadError>;

	/// Writes data to the file given by the path from the buffers.
	fn write(&self, path: &str, buffers: &[u8]) -> Result<(), WriteError>;

	/// Sets the permissions bits of a file.
	fn set_permissions(
		&self,
		path: &str,
		permissions: Permissions,
	) -> Result<(), PermissionsError>;
}

/// Possible errors that can occur when requesting info about a file.
#[derive(Debug)]
pub enum InfoError {}

/// Possible errors that can occur when reading a file.
#[derive(Debug)]
pub enum ReadError {
	/// The file doesn't exist
	NonExistent,
}

/// Possible errors that can occur when reading a file.
#[derive(Debug)]
pub enum WriteError {
	/// The filesystem is read-only
	ReadOnly,
}

/// Possible errors that can occur when setting permissions of a file.
#[derive(Debug)]
pub enum PermissionsError {
	/// The filesystem is read-only
	ReadOnly,
}

/// Structure representing file permissions.
#[derive(Clone, Copy)]
pub struct Permissions(u16);

/// Enum describing possible file types.
pub enum FileType {
	/// A regular file
	Regular,
	/// A directory
	Directory,
}

/// Structure representing an user ID (UID).
pub struct UserID(u32);

/// Structure representing a group ID (GID).
pub struct GroupID(u32);

/// Information about a file
pub struct FileInfo {
	/// The size of the file.
	pub size: u64,
	/// The ID of the user this file belongs to.
	pub user_id: UserID,
	/// The ID of the group this file belongs to.
	pub group_id: GroupID,
	/// The permissions bits of this file.
	pub permissions: Permissions,
	/// The type of the file (called `typ` because `type` is a keyword).
	pub typ: FileType,
}

impl From<u16> for Permissions {
	fn from(raw: u16) -> Self {
		Permissions(raw)
	}
}
