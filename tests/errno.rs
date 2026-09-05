use lunacy::Errno;

#[test]
fn named_and_unknown_errors_round_trip() {
    for error in [
        Errno::EACCES,
        Errno::EAGAIN,
        Errno::EBADF,
        Errno::EEXIST,
        Errno::EFBIG,
        Errno::EINTR,
        Errno::EINVAL,
        Errno::EIO,
        Errno::EMFILE,
        Errno::ENFILE,
        Errno::ENOENT,
        Errno::ENOMEM,
        Errno::ENOSPC,
        Errno::EPIPE,
        Errno::Unknown(0),
        Errno::Unknown(-12345),
    ] {
        assert_eq!(Errno::from_raw(error.to_raw()), error);
    }
}

#[test]
fn native_error_matches_std_without_assuming_a_numeric_value() {
    let error = std::fs::File::open("").unwrap_err();
    assert_eq!(Errno::last(), Errno::ENOENT);
    assert_eq!(Errno::ENOENT.to_raw(), error.raw_os_error().unwrap());
}
#[test]
fn error_text_uses_caller_storage_and_reports_small_buffers() {
    use lunacy::{Errno, strerror_r};
    let mut buffer = [0; 256];
    let text = strerror_r(Errno::ENOENT, &mut buffer).unwrap();
    assert!(!text.to_bytes().is_empty());
    let saved = text.to_owned();
    assert_eq!(strerror_r(Errno::ENOENT, &mut []), Err(Errno::ERANGE));
    assert_eq!(strerror_r(Errno::ENOENT, &mut [0]), Err(Errno::ERANGE));
    assert_eq!(
        strerror_r(Errno::ENOENT, &mut buffer).unwrap(),
        saved.as_c_str()
    );
    let threads: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(|| {
                let mut storage = [0; 256];
                strerror_r(Errno::ENOENT, &mut storage).unwrap().to_owned()
            })
        })
        .collect();
    for thread in threads {
        assert_eq!(thread.join().unwrap(), saved);
    }
}
