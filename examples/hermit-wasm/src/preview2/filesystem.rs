use std::fmt;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use wasmtime::component::{Resource, ResourceTableError};
use wasmtime_wasi_io::bytes::Bytes;
use wasmtime_wasi_io::poll::Pollable;
use wasmtime_wasi_io::streams::{
	DynInputStream, DynOutputStream, InputStream, OutputStream, StreamError, StreamResult,
};

use crate::preview2::WasiCtx;
use crate::preview2::bindings::clocks::wall_clock::Datetime;
use crate::preview2::bindings::filesystem::preopens;
use crate::preview2::bindings::filesystem::types::{
	self, Advice, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry, ErrorCode,
	Filesize, MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
};

/// Directories which are made available to the component, if they exist.
const PREOPENS: &[&str] = &["/tmp", "/root"];

pub enum Descriptor {
	File {
		file: Arc<Mutex<File>>,
		flags: DescriptorFlags,
	},
	Dir {
		path: PathBuf,
		flags: DescriptorFlags,
	},
}

pub struct DirectoryEntryStream;

/// Error of a `wasi:filesystem` operation, either reported to the guest
/// as `error-code` or turned into a trap.
#[derive(Debug)]
pub enum FsError {
	Code(ErrorCode),
	Trap(wasmtime::Error),
}

pub type FsResult<T> = Result<T, FsError>;

impl fmt::Display for FsError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			FsError::Code(code) => write!(f, "{code:?}"),
			FsError::Trap(e) => write!(f, "{e}"),
		}
	}
}

impl std::error::Error for FsError {}

impl From<ErrorCode> for FsError {
	fn from(code: ErrorCode) -> Self {
		FsError::Code(code)
	}
}

impl From<ResourceTableError> for FsError {
	fn from(e: ResourceTableError) -> Self {
		FsError::Trap(e.into())
	}
}

impl From<io::Error> for FsError {
	fn from(e: io::Error) -> Self {
		use io::ErrorKind::*;

		FsError::Code(match e.kind() {
			NotFound => ErrorCode::NoEntry,
			PermissionDenied => ErrorCode::Access,
			AlreadyExists => ErrorCode::Exist,
			InvalidInput | InvalidData => ErrorCode::Invalid,
			Unsupported => ErrorCode::Unsupported,
			IsADirectory => ErrorCode::IsDirectory,
			NotADirectory => ErrorCode::NotDirectory,
			DirectoryNotEmpty => ErrorCode::NotEmpty,
			ReadOnlyFilesystem => ErrorCode::ReadOnly,
			StorageFull => ErrorCode::InsufficientSpace,
			FileTooLarge => ErrorCode::FileTooLarge,
			Interrupted => ErrorCode::Interrupted,
			WouldBlock => ErrorCode::WouldBlock,
			OutOfMemory => ErrorCode::InsufficientMemory,
			_ => ErrorCode::Io,
		})
	}
}

fn unsupported<T>() -> FsResult<T> {
	Err(ErrorCode::Unsupported.into())
}

/// Joins `path` to `base` without leaving `base`.
fn resolve(base: &Path, path: &str) -> FsResult<PathBuf> {
	let mut resolved = base.to_path_buf();
	let mut depth = 0usize;
	for component in Path::new(path).components() {
		match component {
			Component::Normal(name) => {
				resolved.push(name);
				depth += 1;
			}
			Component::CurDir => {}
			Component::ParentDir if depth > 0 => {
				resolved.pop();
				depth -= 1;
			}
			Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
				return Err(ErrorCode::NotPermitted.into());
			}
		}
	}
	Ok(resolved)
}

fn descriptor_type(metadata: &Metadata) -> DescriptorType {
	let file_type = metadata.file_type();
	if file_type.is_dir() {
		DescriptorType::Directory
	} else if file_type.is_file() {
		DescriptorType::RegularFile
	} else if file_type.is_symlink() {
		DescriptorType::SymbolicLink
	} else {
		DescriptorType::Unknown
	}
}

fn datetime(time: io::Result<SystemTime>) -> Option<Datetime> {
	let duration = time.ok()?.duration_since(UNIX_EPOCH).ok()?;
	Some(Datetime {
		seconds: duration.as_secs(),
		nanoseconds: duration.subsec_nanos(),
	})
}

fn descriptor_stat(metadata: &Metadata) -> DescriptorStat {
	DescriptorStat {
		type_: descriptor_type(metadata),
		link_count: 1,
		size: metadata.len(),
		data_access_timestamp: datetime(metadata.accessed()),
		data_modification_timestamp: datetime(metadata.modified()),
		status_change_timestamp: None,
	}
}

struct FileInputStream {
	file: Arc<Mutex<File>>,
	position: u64,
}

#[wasmtime_wasi_io::async_trait]
impl Pollable for FileInputStream {
	async fn ready(&mut self) {}
}

impl InputStream for FileInputStream {
	fn read(&mut self, size: usize) -> StreamResult<Bytes> {
		let mut file = self.file.lock().unwrap();
		let mut buf = vec![0; size];
		let n = file
			.seek(SeekFrom::Start(self.position))
			.and_then(|_| file.read(&mut buf))
			.map_err(|e| StreamError::LastOperationFailed(e.into()))?;
		if n == 0 && size > 0 {
			return Err(StreamError::Closed);
		}
		buf.truncate(n);
		self.position += n as u64;
		Ok(buf.into())
	}
}

struct FileOutputStream {
	file: Arc<Mutex<File>>,
	/// Write position, `None` appends to the end of the file
	position: Option<u64>,
}

#[wasmtime_wasi_io::async_trait]
impl Pollable for FileOutputStream {
	async fn ready(&mut self) {}
}

impl OutputStream for FileOutputStream {
	fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
		let mut file = self.file.lock().unwrap();
		let pos = match self.position {
			Some(position) => SeekFrom::Start(position),
			None => SeekFrom::End(0),
		};
		file.seek(pos)
			.and_then(|_| file.write_all(&bytes))
			.map_err(|e| StreamError::LastOperationFailed(e.into()))?;
		if let Some(position) = &mut self.position {
			*position += bytes.len() as u64;
		}
		Ok(())
	}

	fn flush(&mut self) -> StreamResult<()> {
		self.file
			.lock()
			.unwrap()
			.flush()
			.map_err(|e| StreamError::LastOperationFailed(e.into()))
	}

	fn check_write(&mut self) -> StreamResult<usize> {
		Ok(1024 * 1024)
	}
}

impl WasiCtx {
	fn dir_path(&self, fd: &Resource<Descriptor>) -> FsResult<PathBuf> {
		match self.table.get(fd)? {
			Descriptor::Dir { path, .. } => Ok(path.clone()),
			Descriptor::File { .. } => Err(ErrorCode::NotDirectory.into()),
		}
	}

	fn file(
		&self,
		fd: &Resource<Descriptor>,
		required: DescriptorFlags,
	) -> FsResult<Arc<Mutex<File>>> {
		match self.table.get(fd)? {
			Descriptor::File { file, flags } if flags.contains(required) => Ok(file.clone()),
			Descriptor::File { .. } => Err(ErrorCode::BadDescriptor.into()),
			Descriptor::Dir { .. } => Err(ErrorCode::IsDirectory.into()),
		}
	}
}

impl types::Host for WasiCtx {
	fn convert_error_code(&mut self, err: FsError) -> wasmtime::Result<ErrorCode> {
		match err {
			FsError::Code(code) => Ok(code),
			FsError::Trap(e) => Err(e),
		}
	}

	fn filesystem_error_code(
		&mut self,
		_err: Resource<wasmtime::Error>,
	) -> wasmtime::Result<Option<ErrorCode>> {
		Ok(None)
	}
}

impl types::HostDescriptor for WasiCtx {
	fn read_via_stream(
		&mut self,
		fd: Resource<Descriptor>,
		offset: Filesize,
	) -> FsResult<Resource<DynInputStream>> {
		let file = self.file(&fd, DescriptorFlags::READ)?;
		let stream: DynInputStream = Box::new(FileInputStream {
			file,
			position: offset,
		});
		Ok(self.table.push(stream)?)
	}

	fn write_via_stream(
		&mut self,
		fd: Resource<Descriptor>,
		offset: Filesize,
	) -> FsResult<Resource<DynOutputStream>> {
		let file = self.file(&fd, DescriptorFlags::WRITE)?;
		let stream: DynOutputStream = Box::new(FileOutputStream {
			file,
			position: Some(offset),
		});
		Ok(self.table.push(stream)?)
	}

	fn append_via_stream(
		&mut self,
		fd: Resource<Descriptor>,
	) -> FsResult<Resource<DynOutputStream>> {
		let file = self.file(&fd, DescriptorFlags::WRITE)?;
		let stream: DynOutputStream = Box::new(FileOutputStream {
			file,
			position: None,
		});
		Ok(self.table.push(stream)?)
	}

	fn advise(
		&mut self,
		_fd: Resource<Descriptor>,
		_offset: Filesize,
		_len: Filesize,
		_advice: Advice,
	) -> FsResult<()> {
		unsupported()
	}

	fn sync_data(&mut self, fd: Resource<Descriptor>) -> FsResult<()> {
		match self.table.get(&fd)? {
			Descriptor::File { file, flags } if flags.contains(DescriptorFlags::WRITE) => {
				Ok(file.lock().unwrap().sync_data()?)
			}
			Descriptor::File { .. } | Descriptor::Dir { .. } => Ok(()),
		}
	}

	fn get_flags(&mut self, fd: Resource<Descriptor>) -> FsResult<DescriptorFlags> {
		match self.table.get(&fd)? {
			Descriptor::File { flags, .. } | Descriptor::Dir { flags, .. } => Ok(*flags),
		}
	}

	fn get_type(&mut self, fd: Resource<Descriptor>) -> FsResult<DescriptorType> {
		match self.table.get(&fd)? {
			Descriptor::File { file, .. } => Ok(descriptor_type(&file.lock().unwrap().metadata()?)),
			Descriptor::Dir { .. } => Ok(DescriptorType::Directory),
		}
	}

	fn set_size(&mut self, _fd: Resource<Descriptor>, _size: Filesize) -> FsResult<()> {
		unsupported()
	}

	fn set_times(
		&mut self,
		_fd: Resource<Descriptor>,
		_atim: NewTimestamp,
		_mtim: NewTimestamp,
	) -> FsResult<()> {
		unsupported()
	}

	fn read(
		&mut self,
		fd: Resource<Descriptor>,
		len: Filesize,
		offset: Filesize,
	) -> FsResult<(Vec<u8>, bool)> {
		let file = self.file(&fd, DescriptorFlags::READ)?;
		let mut file = file.lock().unwrap();
		file.seek(SeekFrom::Start(offset))?;
		let mut data = vec![];
		(&mut *file).take(len).read_to_end(&mut data)?;
		let eof = (data.len() as Filesize) < len;
		Ok((data, eof))
	}

	fn write(
		&mut self,
		fd: Resource<Descriptor>,
		buf: Vec<u8>,
		offset: Filesize,
	) -> FsResult<Filesize> {
		let file = self.file(&fd, DescriptorFlags::WRITE)?;
		let mut file = file.lock().unwrap();
		file.seek(SeekFrom::Start(offset))?;
		file.write_all(&buf)?;
		Ok(buf.len() as Filesize)
	}

	fn read_directory(
		&mut self,
		_fd: Resource<Descriptor>,
	) -> FsResult<Resource<DirectoryEntryStream>> {
		unsupported()
	}

	fn sync(&mut self, fd: Resource<Descriptor>) -> FsResult<()> {
		match self.table.get(&fd)? {
			Descriptor::File { file, flags } if flags.contains(DescriptorFlags::WRITE) => {
				Ok(file.lock().unwrap().sync_all()?)
			}
			Descriptor::File { .. } | Descriptor::Dir { .. } => Ok(()),
		}
	}

	fn create_directory_at(&mut self, _fd: Resource<Descriptor>, _path: String) -> FsResult<()> {
		unsupported()
	}

	fn stat(&mut self, fd: Resource<Descriptor>) -> FsResult<DescriptorStat> {
		let metadata = match self.table.get(&fd)? {
			Descriptor::File { file, .. } => file.lock().unwrap().metadata()?,
			Descriptor::Dir { path, .. } => fs::metadata(path)?,
		};
		Ok(descriptor_stat(&metadata))
	}

	fn stat_at(
		&mut self,
		fd: Resource<Descriptor>,
		path_flags: PathFlags,
		path: String,
	) -> FsResult<DescriptorStat> {
		let path = resolve(&self.dir_path(&fd)?, &path)?;
		let metadata = if path_flags.contains(PathFlags::SYMLINK_FOLLOW) {
			fs::metadata(path)?
		} else {
			fs::symlink_metadata(path)?
		};
		Ok(descriptor_stat(&metadata))
	}

	fn set_times_at(
		&mut self,
		_fd: Resource<Descriptor>,
		_path_flags: PathFlags,
		_path: String,
		_atim: NewTimestamp,
		_mtim: NewTimestamp,
	) -> FsResult<()> {
		unsupported()
	}

	fn link_at(
		&mut self,
		_fd: Resource<Descriptor>,
		_old_path_flags: PathFlags,
		_old_path: String,
		_new_fd: Resource<Descriptor>,
		_new_path: String,
	) -> FsResult<()> {
		unsupported()
	}

	fn open_at(
		&mut self,
		fd: Resource<Descriptor>,
		_path_flags: PathFlags,
		path: String,
		open_flags: OpenFlags,
		flags: DescriptorFlags,
	) -> FsResult<Resource<Descriptor>> {
		let path = resolve(&self.dir_path(&fd)?, &path)?;
		let is_dir = fs::metadata(&path).is_ok_and(|metadata| metadata.is_dir());

		let descriptor = if open_flags.contains(OpenFlags::DIRECTORY) || is_dir {
			if !is_dir {
				return Err(ErrorCode::NotDirectory.into());
			}
			if flags.contains(DescriptorFlags::WRITE) {
				return Err(ErrorCode::IsDirectory.into());
			}
			Descriptor::Dir { path, flags }
		} else {
			let file = OpenOptions::new()
				.read(flags.contains(DescriptorFlags::READ))
				.write(flags.contains(DescriptorFlags::WRITE))
				.create(open_flags.contains(OpenFlags::CREATE))
				.create_new(open_flags.contains(OpenFlags::EXCLUSIVE))
				.truncate(open_flags.contains(OpenFlags::TRUNCATE))
				.open(path)?;
			Descriptor::File {
				file: Arc::new(Mutex::new(file)),
				flags,
			}
		};

		Ok(self.table.push(descriptor)?)
	}

	fn readlink_at(&mut self, _fd: Resource<Descriptor>, _path: String) -> FsResult<String> {
		unsupported()
	}

	fn remove_directory_at(&mut self, _fd: Resource<Descriptor>, _path: String) -> FsResult<()> {
		unsupported()
	}

	fn rename_at(
		&mut self,
		_fd: Resource<Descriptor>,
		_old_path: String,
		_new_fd: Resource<Descriptor>,
		_new_path: String,
	) -> FsResult<()> {
		unsupported()
	}

	fn symlink_at(
		&mut self,
		_fd: Resource<Descriptor>,
		_old_path: String,
		_new_path: String,
	) -> FsResult<()> {
		unsupported()
	}

	fn unlink_file_at(&mut self, fd: Resource<Descriptor>, path: String) -> FsResult<()> {
		let path = resolve(&self.dir_path(&fd)?, &path)?;
		fs::remove_file(path)?;
		Ok(())
	}

	fn is_same_object(
		&mut self,
		_fd: Resource<Descriptor>,
		_other: Resource<Descriptor>,
	) -> wasmtime::Result<bool> {
		Ok(false)
	}

	fn metadata_hash(&mut self, _fd: Resource<Descriptor>) -> FsResult<MetadataHashValue> {
		unsupported()
	}

	fn metadata_hash_at(
		&mut self,
		_fd: Resource<Descriptor>,
		_path_flags: PathFlags,
		_path: String,
	) -> FsResult<MetadataHashValue> {
		unsupported()
	}

	fn drop(&mut self, fd: Resource<Descriptor>) -> wasmtime::Result<()> {
		self.table.delete(fd)?;
		Ok(())
	}
}

impl types::HostDirectoryEntryStream for WasiCtx {
	fn read_directory_entry(
		&mut self,
		_stream: Resource<DirectoryEntryStream>,
	) -> FsResult<Option<DirectoryEntry>> {
		unsupported()
	}

	fn drop(&mut self, stream: Resource<DirectoryEntryStream>) -> wasmtime::Result<()> {
		self.table.delete(stream)?;
		Ok(())
	}
}

impl preopens::Host for WasiCtx {
	fn get_directories(&mut self) -> wasmtime::Result<Vec<(Resource<Descriptor>, String)>> {
		let mut directories = Vec::new();
		for name in PREOPENS.iter().filter(|name| Path::new(name).is_dir()) {
			let descriptor = Descriptor::Dir {
				path: PathBuf::from(name),
				flags: DescriptorFlags::READ | DescriptorFlags::MUTATE_DIRECTORY,
			};
			directories.push((self.table.push(descriptor)?, name.to_string()));
		}
		Ok(directories)
	}
}
