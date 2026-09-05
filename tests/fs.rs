use lunacy::{Errno, FileType, Mode, OpenFlags, Whence};
use std::{
    ffi::{CStr, CString},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, symlink},
    },
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "lunacy-fs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self, name: &str) -> CString {
        cpath(&self.0.join(name))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn cpath(path: &Path) -> CString {
    CString::new(path.as_os_str().as_bytes()).unwrap()
}
fn private_mode() -> Mode {
    Mode::irusr() | Mode::iwusr()
}

#[test]
fn file_creation_seek_shared_offset_truncate_and_unlink() {
    let temp = Temp::new();
    let path = temp.path("data");
    let flags = OpenFlags::rdwr() | OpenFlags::creat() | OpenFlags::excl();
    let file = lunacy::open(&path, flags, private_mode()).unwrap();
    assert_eq!(
        lunacy::open(&path, flags, private_mode()).err(),
        Some(Errno::EEXIST)
    );
    assert_eq!(lunacy::write(file.as_fd(), b"abcdef"), Ok(6));
    let duplicate = lunacy::dup(file.as_fd()).unwrap();
    assert_eq!(lunacy::lseek(duplicate.as_fd(), -3, Whence::Cur), Ok(3));
    let mut buffer = [0; 3];
    assert_eq!(lunacy::read(file.as_fd(), &mut buffer), Ok(3));
    assert_eq!(&buffer, b"def");
    lunacy::ftruncate(file.as_fd(), 2).unwrap();
    assert_eq!(lunacy::lseek(file.as_fd(), 0, Whence::Cur), Ok(6));
    assert_eq!(lunacy::fstat(file.as_fd()).unwrap().st_size, 2);
    lunacy::ftruncate(file.as_fd(), 8).unwrap();
    assert_eq!(lunacy::lseek(file.as_fd(), -2, Whence::End), Ok(6));
    assert_eq!(lunacy::read(file.as_fd(), &mut buffer), Ok(2));
    assert_eq!(&buffer[..2], &[0, 0]);
    lunacy::unlink(&path).unwrap();
    assert_eq!(lunacy::stat(&path), Err(Errno::ENOENT));
    assert_eq!(lunacy::fstat(file.as_fd()).unwrap().st_size, 8);
    lunacy::close(file).unwrap();
    lunacy::close(duplicate).unwrap();
}

#[test]
fn stat_fields_and_symlink_behavior_match_native_metadata() {
    let temp = Temp::new();
    let path = temp.path("data");
    std::fs::write(temp.0.join("data"), b"metadata").unwrap();
    symlink("data", temp.0.join("link")).unwrap();
    let actual = lunacy::stat(&path).unwrap();
    let native = std::fs::metadata(temp.0.join("data")).unwrap();
    assert_eq!(actual.st_dev, native.dev());
    assert_eq!(actual.st_ino, native.ino());
    assert_eq!(actual.st_nlink, native.nlink());
    assert_eq!(actual.st_uid, u64::from(native.uid()));
    assert_eq!(actual.st_gid, u64::from(native.gid()));
    assert_eq!(actual.st_rdev, native.rdev());
    assert_eq!(actual.st_mode.to_raw(), native.mode());
    assert_eq!(actual.st_size as u64, native.size());
    assert_eq!(
        (actual.st_atim.tv_sec, actual.st_atim.tv_nsec),
        (native.atime(), native.atime_nsec())
    );
    assert_eq!(
        (actual.st_mtim.tv_sec, actual.st_mtim.tv_nsec),
        (native.mtime(), native.mtime_nsec())
    );
    assert_eq!(
        (actual.st_ctim.tv_sec, actual.st_ctim.tv_nsec),
        (native.ctime(), native.ctime_nsec())
    );
    assert_eq!(actual.st_mode.file_type(), FileType::Regular);
    assert_eq!(lunacy::stat(&temp.path("link")).unwrap(), actual);
    assert_eq!(
        lunacy::lstat(&temp.path("link"))
            .unwrap()
            .st_mode
            .file_type(),
        FileType::Symlink
    );
    assert_eq!(
        lunacy::stat(&cpath(&temp.0)).unwrap().st_mode.file_type(),
        FileType::Directory
    );
    assert_eq!(
        lunacy::open(
            &temp.path("link"),
            OpenFlags::rdonly() | OpenFlags::nofollow(),
            Mode::empty()
        )
        .err(),
        Some(Errno::ELOOP)
    );
}

#[test]
fn openat_follows_directory_descriptor_after_rename() {
    let temp = Temp::new();
    let old = temp.path("old");
    let new = temp.path("new");
    lunacy::mkdir(&old, private_mode() | Mode::ixusr()).unwrap();
    let directory = lunacy::open(
        &old,
        OpenFlags::rdonly() | OpenFlags::directory(),
        Mode::empty(),
    )
    .unwrap();
    lunacy::rename(&old, &new).unwrap();
    let file = lunacy::openat(
        Some(directory.as_fd()),
        c"child",
        OpenFlags::rdwr() | OpenFlags::creat(),
        private_mode(),
    )
    .unwrap();
    lunacy::write(file.as_fd(), b"relative").unwrap();
    assert_eq!(
        std::fs::read(temp.0.join("new/child")).unwrap(),
        b"relative"
    );
    let absolute = lunacy::openat(
        None,
        &temp.path("new/child"),
        OpenFlags::rdonly(),
        Mode::empty(),
    )
    .unwrap();
    assert_eq!(
        lunacy::fstat(absolute.as_fd()).unwrap().st_ino,
        lunacy::fstat(file.as_fd()).unwrap().st_ino
    );
    assert_eq!(lunacy::rmdir(&new), Err(Errno::ENOTEMPTY));
    lunacy::unlink(&temp.path("new/child")).unwrap();
    lunacy::rmdir(&new).unwrap();
    assert_eq!(lunacy::stat(&new), Err(Errno::ENOENT));
}

#[test]
fn append_truncation_and_errors_preserve_libc_behavior() {
    let temp = Temp::new();
    let path = temp.path("append");
    std::fs::write(temp.0.join("append"), b"old").unwrap();
    let file = lunacy::open(
        &path,
        OpenFlags::wronly() | OpenFlags::append(),
        Mode::empty(),
    )
    .unwrap();
    lunacy::lseek(file.as_fd(), 0, Whence::Set).unwrap();
    assert_eq!(lunacy::write(file.as_fd(), b"!"), Ok(1));
    assert_eq!(std::fs::read(temp.0.join("append")).unwrap(), b"old!");
    assert_eq!(lunacy::ftruncate(file.as_fd(), -1), Err(Errno::EINVAL));
    assert_eq!(
        lunacy::lseek(file.as_fd(), -1, Whence::Set),
        Err(Errno::EINVAL)
    );
    let truncated = lunacy::open(
        &path,
        OpenFlags::wronly() | OpenFlags::trunc(),
        Mode::empty(),
    )
    .unwrap();
    assert_eq!(lunacy::fstat(truncated.as_fd()).unwrap().st_size, 0);
    assert_eq!(
        lunacy::open(&temp.path("absent"), OpenFlags::rdonly(), Mode::empty()).err(),
        Some(Errno::ENOENT)
    );
    let (read, _) = lunacy::pipe().unwrap();
    assert_eq!(
        lunacy::lseek(read.as_fd(), 0, Whence::Set),
        Err(Errno::ESPIPE)
    );
    assert_eq!(
        lunacy::fstat(read.as_fd()).unwrap().st_mode.file_type(),
        FileType::Fifo
    );
    assert!(!lunacy::isatty(read.as_fd()).unwrap());
}

#[test]
fn mkstemp_rewrites_template_and_returns_unique_owned_file() {
    let temp = Temp::new();
    let original = temp.path("temp-XXXXXX").into_bytes_with_nul();
    let mut first = original.clone();
    let mut second = original;
    let file = lunacy::mkstemp(&mut first).unwrap();
    let other = lunacy::mkstemp(&mut second).unwrap();
    assert_ne!(first, second);
    let name = CStr::from_bytes_with_nul(&first).unwrap();
    assert_eq!(
        lunacy::fstat(file.as_fd()).unwrap().st_ino,
        lunacy::stat(name).unwrap().st_ino
    );
    assert_eq!(
        lunacy::fstat(file.as_fd()).unwrap().st_mode.to_raw() & 0o077,
        0
    );
    assert_eq!(lunacy::write(file.as_fd(), b"temporary"), Ok(9));
    lunacy::close(file).unwrap();
    assert_eq!(lunacy::stat(name).unwrap().st_size, 9);
    lunacy::unlink(name).unwrap();
    lunacy::close(other).unwrap();
    lunacy::unlink(CStr::from_bytes_with_nul(&second).unwrap()).unwrap();
    for invalid in [
        b"no-suffix\0".to_vec(),
        b"XXXXXX".to_vec(),
        b"\0XXXXXX\0".to_vec(),
        vec![],
    ] {
        assert_eq!(
            lunacy::mkstemp(&mut invalid.clone()).err(),
            Some(Errno::EINVAL)
        );
    }
}

#[test]
fn isatty_recognizes_a_pseudoterminal() {
    // SAFETY: posix_openpt has no memory arguments and creates a fresh descriptor.
    let raw = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
    assert!(raw >= 0);
    // SAFETY: Transfer exclusive ownership of the newly opened descriptor.
    let terminal = unsafe { lunacy::OwnedFd::from_raw_fd(raw) };
    assert!(lunacy::isatty(terminal.as_fd()).unwrap());
}

#[test]
fn native_flags_and_seek_origins_are_resolved_by_c() {
    assert_eq!(OpenFlags::rdonly().to_raw(), libc::O_RDONLY);
    assert_eq!(OpenFlags::wronly().to_raw(), libc::O_WRONLY);
    assert_eq!(OpenFlags::rdwr().to_raw(), libc::O_RDWR);
    assert_eq!(
        (OpenFlags::creat() | OpenFlags::excl() | OpenFlags::cloexec()).to_raw(),
        libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC
    );
    assert_eq!(
        (Mode::irusr() | Mode::iwusr() | Mode::ixusr()).to_raw(),
        libc::S_IRWXU
    );
    assert_eq!(Whence::Set.to_raw(), libc::SEEK_SET);
    assert_eq!(Whence::Cur.to_raw(), libc::SEEK_CUR);
    assert_eq!(Whence::End.to_raw(), libc::SEEK_END);
}
