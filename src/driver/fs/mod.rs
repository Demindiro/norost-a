//! Drivers for filesystems

mod cpio;
mod initramfs;

/// Trait implemented by all filesystem drivers
///
/// It defines a number of methods to load data from and store data in a backing store.
pub trait FileSystem {
	/// Returns information about the file with the given inode
	fn info(&self, inode: INode) -> Result<FileInfo, InfoError>;

	/// Reads data from the file given by the inode and writes it into the given buffers.
	fn read(&self, inode: INode, buffers: &mut [u8]) -> Result<(), ReadError>;

	/// Writes data to the file given by the inode from the buffers.
	fn write(&self, inode: INode, buffers: &[u8]) -> Result<(), WriteError>;

	/// Sets the permissions bits of a file.
	fn set_permissions(
		&self,
		inode: INode,
		permissions: Permissions,
	) -> Result<(), PermissionsError>;
}

/// Possible errors that can occur when requesting info about a file.
pub enum InfoError {}

/// Possible errors that can occur when reading a file.
pub enum ReadError {}

/// Possible errors that can occur when reading a file.
pub enum WriteError {}

/// Possible errors that can occur when setting permissions of a file.
pub enum PermissionsError {}

/// Structure representing an inode.
pub struct INode(u64);

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
	/// The inode of the file.
	inode: INode,
	/// The size of the file.
	size: u64,
	/// The ID of the user this file belongs to.
	user_id: UserID,
	/// The ID of the group this file belongs to.
	group_id: GroupID,
	/// The permissions bits of this file.
	permissions: Permissions,
	/// The type of the file (called `typ` because `type` is a keyword).
	typ: FileType,
}

impl From<u16> for Permissions {
	fn from(raw: u16) -> Self {
		Permissions(raw)
	}
}
