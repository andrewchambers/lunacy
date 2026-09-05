use lunacy::{
    Errno,
    fs::{self, Mode, OpenFlags},
};
use std::{ffi::CString, os::unix::ffi::OsStrExt};

// Process cwd is global. This executable uses no threaded test harness.
fn main() {
    let original = std::env::current_dir().unwrap();
    let temporary = std::env::temp_dir().join(format!("lunacy-cwd-{}", std::process::id()));
    std::fs::create_dir(&temporary).unwrap();
    let path = CString::new(temporary.as_os_str().as_bytes()).unwrap();
    fs::chdir(&path).unwrap();
    let mut buffer = vec![
        0;
        std::fs::canonicalize(&temporary)
            .unwrap()
            .as_os_str()
            .as_bytes()
            .len()
            + 1
    ];
    assert_eq!(
        fs::getcwd(&mut buffer).unwrap().to_bytes(),
        std::env::current_dir().unwrap().as_os_str().as_bytes()
    );
    assert_eq!(fs::getcwd(&mut []), Err(Errno::ERANGE));
    assert_eq!(fs::getcwd(&mut [0]), Err(Errno::ERANGE));
    let relative = fs::openat(
        None,
        c"relative",
        OpenFlags::creat() | OpenFlags::wronly(),
        Mode::irusr() | Mode::iwusr(),
    )
    .unwrap();
    assert_eq!(
        fs::fstat(relative.as_fd()).unwrap().st_ino,
        fs::stat(c"relative").unwrap().st_ino
    );
    assert_eq!(fs::chdir(c"relative"), Err(Errno::ENOTDIR));
    fs::chdir(&CString::new(original.as_os_str().as_bytes()).unwrap()).unwrap();
    std::fs::remove_dir_all(temporary).unwrap();
}
