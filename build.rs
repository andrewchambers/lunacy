fn main() {
    assert!(
        std::env::var_os("CARGO_CFG_UNIX").is_some(),
        "lunacy currently requires a POSIX host libc"
    );
    println!("cargo:rerun-if-changed=src/shim.c");
    println!("cargo:rerun-if-changed=src/net.c");
    println!("cargo:rerun-if-changed=src/readiness.c");
    println!("cargo:rerun-if-changed=src/abi.h");
    println!("cargo:rerun-if-changed=src/compat.h");
    println!("cargo:rerun-if-changed=src/time.c");
    println!("cargo:rerun-if-changed=src/fs.c");
    println!("cargo:rerun-if-changed=src/process.c");
    let pthread = std::env::var_os("CARGO_FEATURE_PTHREAD").is_some();
    let mut build = cc::Build::new();
    build
        .file("src/shim.c")
        .file("src/net.c")
        .file("src/readiness.c")
        .file("src/time.c")
        .file("src/fs.c")
        .file("src/process.c")
        .std("c99")
        .warnings(true);
    if pthread {
        println!("cargo:rerun-if-changed=src/pthread.c");
        build.file("src/pthread.c").flag_if_supported("-pthread");
    }
    build.compile("lunacy_shim");
    if pthread {
        println!("cargo:rustc-link-lib=pthread");
    }
}
