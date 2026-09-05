use lunacy::args::Args;

#[test]
fn arguments_preserve_c_string_bytes_and_bounds() {
    let strings = [c"program", c"", c"non-utf8-\xff"];
    let pointers: Vec<_> = strings.iter().map(|s| s.as_ptr()).collect();
    // SAFETY: All pointers are valid immutable static strings, and pointers
    // remains alive and unchanged for the entire view's use.
    let args = unsafe { Args::from_raw(pointers.len(), pointers.as_ptr()) };
    assert_eq!(args.len(), 3);
    assert!(!args.is_empty());
    for (index, text) in strings.iter().enumerate() {
        assert_eq!(args.get(index), Some(*text));
    }
    assert_eq!(args.get(3), None);
    assert_eq!(args.get(usize::MAX), None);
}

#[test]
fn zero_argc_accepts_null_without_dereferencing() {
    // SAFETY: The zero-length argument vector requires no pointer storage.
    let args = unsafe { Args::from_raw(0, core::ptr::null()) };
    assert!(args.is_empty());
    assert_eq!(args.get(0), None);
}
