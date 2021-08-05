//! # Example filesystem driver using FAT


pub fn init<B>(mut backend: B) -> fatfs::FileSystem<B, fatfs::NullTimeProvider, fatfs::LossyOemCpConverter>
where
	B: fatfs::ReadWriteSeek
{
	let mut fvo = fatfs::FormatVolumeOptions::new()
		.volume_label(*b"DUX ROOT\0\0\0")
		.volume_id(100117120)
		.max_root_dir_entries(16)
		;
	let ret = fatfs::format_volume(&mut backend, fvo);
	ret.unwrap();
	let mut fs = fatfs::FileSystem::new(backend, fatfs::FsOptions::new()).unwrap();
	fs.root_dir().create_file("avada");
	fs.root_dir().create_file("kedavra");
	fs.root_dir().create_file("ded");
	fs
}

pub fn open<B>(mut backend: B) -> fatfs::FileSystem<B, fatfs::NullTimeProvider, fatfs::LossyOemCpConverter>
where
	B: fatfs::ReadWriteSeek,
{
	fatfs::FileSystem::new(backend, fatfs::FsOptions::new()).unwrap()
}
