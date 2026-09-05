use lunacy::{Errno, closedir, opendir, readdir};
use std::{
    collections::BTreeSet,
    ffi::CString,
    os::unix::ffi::OsStrExt,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "lunacy-dir-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn name(&self) -> CString {
        CString::new(self.0.as_os_str().as_bytes()).unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn entries_preserve_names_and_distinguish_eof_from_stale_errno() {
    let temp = Temp::new();
    let names = [b"file".to_vec(), b"non-utf8-\xff".to_vec(), vec![b'x'; 200]];
    for name in &names {
        std::fs::write(temp.0.join(std::ffi::OsStr::from_bytes(name)), b"").unwrap();
    }
    let mut directory = opendir(&temp.name()).unwrap();
    let mut seen = BTreeSet::new();
    while let Some(entry) = readdir(&mut directory).unwrap() {
        assert_ne!(entry.d_ino, 0);
        seen.insert(entry.d_name.to_owned());
    }
    for name in &names {
        assert!(seen.contains(&CString::new(name.clone()).unwrap()));
    }
    assert!(seen.contains(c"."));
    assert!(seen.contains(c".."));
    assert_eq!(
        lunacy::stat(c"/lunacy-test-path-that-does-not-exist"),
        Err(Errno::ENOENT)
    );
    assert!(readdir(&mut directory).unwrap().is_none());
    closedir(directory).unwrap();
    // Owned names survive reads and closing the native stream.
    assert_eq!(seen.len(), names.len() + 2);
}

#[test]
fn separate_streams_can_be_read_concurrently() {
    let temp = Temp::new();
    std::fs::write(temp.0.join("file"), b"").unwrap();
    let streams: Vec<_> = (0..8).map(|_| opendir(&temp.name()).unwrap()).collect();
    let threads: Vec<_> = streams
        .into_iter()
        .map(|mut stream| {
            std::thread::spawn(move || {
                let mut names = BTreeSet::new();
                while let Some(entry) = readdir(&mut stream).unwrap() {
                    names.insert(entry.d_name.to_owned());
                }
                names
            })
        })
        .collect();
    for thread in threads {
        assert_eq!(thread.join().unwrap().len(), 3);
    }
}

#[test]
fn opening_regular_file_or_missing_path_reports_native_error() {
    let temp = Temp::new();
    let path = temp.0.join("file");
    std::fs::write(&path, b"").unwrap();
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(opendir(&name).err(), Some(Errno::ENOTDIR));
    std::fs::remove_file(&path).unwrap();
    assert_eq!(opendir(&name).err(), Some(Errno::ENOENT));
}
