use std::fmt;
use std::fs::{FileType, Metadata};
#[cfg(target_os = "hermit")]
use std::os::hermit::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use libc::mode_t;

pub struct MetadataDisplay<'a>(&'a Metadata);

impl fmt::Display for MetadataDisplay<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let file_type = self.0.file_type().display();
		let mode = self.0.mode();
		write!(f, "{file_type}")
	}
}

pub trait MetadataExt {
	fn display(&self) -> MetadataDisplay<'_>;
}

impl MetadataExt for Metadata {
	fn display(&self) -> MetadataDisplay<'_> {
		MetadataDisplay(self)
	}
}

#[derive(Clone, Copy, Debug)]
enum FileTypeDisplay {
	Dir,
	File,
	Symlink,
	Unknown,
}

impl FileTypeDisplay {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Dir => "dir",
			Self::File => "file",
			Self::Symlink => "symlink",
			Self::Unknown => "unknown",
		}
	}
}

impl fmt::Display for FileTypeDisplay {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

trait FileTypeExt {
	fn display(&self) -> FileTypeDisplay;
}

impl FileTypeExt for FileType {
	fn display(&self) -> FileTypeDisplay {
		match self {
			this if this.is_dir() => FileTypeDisplay::Dir,
			this if this.is_file() => FileTypeDisplay::File,
			this if this.is_symlink() => FileTypeDisplay::Symlink,
			_ => FileTypeDisplay::Unknown,
		}
	}
}

struct ModeDisplay(mode_t);

trait ModeExt {
	fn display_mode(self) -> ModeDisplay;
}

impl ModeExt for mode_t {
	fn display_mode(self) -> ModeDisplay {
		ModeDisplay(self)
	}
}
