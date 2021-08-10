//! # Example filesystem driver using FAT

pub fn init<B>(
	mut backend: B,
) -> fatfs::FileSystem<B, fatfs::NullTimeProvider, fatfs::LossyOemCpConverter>
where
	B: fatfs::ReadWriteSeek,
{
	let fvo = fatfs::FormatVolumeOptions::new()
		.volume_label(*b"DUX ROOT\0\0\0")
		.volume_id(100117120)
		.max_root_dir_entries(16);
	fatfs::format_volume(&mut backend, fvo).unwrap();
	let fs = fatfs::FileSystem::new(backend, fatfs::FsOptions::new()).unwrap();
	use fatfs::Write;
	fs.root_dir()
		.create_file("avada")
		.unwrap()
		.write(b"Yes, this is indeed a reference.\n")
		.unwrap();
	fs.root_dir()
		.create_file("kedavra")
		.unwrap()
		.write(b"It is very much a reference.\n")
		.unwrap();
	fs.root_dir()
		.create_file("ded")
		.unwrap()
		.write(b"This is sorta a reference? Perhaps not.\n")
		.unwrap();
	fs
}

pub fn open<B>(
	backend: B,
) -> Result<
	fatfs::FileSystem<B, fatfs::NullTimeProvider, fatfs::LossyOemCpConverter>,
	fatfs::Error<<B as fatfs::IoBase>::Error>,
>
where
	B: fatfs::ReadWriteSeek,
{
	fatfs::FileSystem::new(backend, fatfs::FsOptions::new())
}
