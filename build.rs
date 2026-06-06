use std::env;

fn main() {
    println!("cargo:rerun-if-changed=src/errno.c");
    println!("cargo:rerun-if-changed=src/net.c");
    println!("cargo:rerun-if-changed=src/sys.c");
    println!("cargo:rerun-if-changed=src/time.c");
    println!("cargo:rerun-if-changed=src/pthread.c");
    println!("cargo:rustc-link-lib=c");
    println!("cargo:rustc-link-lib=gcc_s");

    let pthread_enabled = env::var_os("CARGO_FEATURE_PTHREAD").is_some();
    if pthread_enabled {
        println!("cargo:rustc-link-lib=pthread");
    }

    let mut build = cc::Build::new();
    build
        .file("src/errno.c")
        .file("src/net.c")
        .file("src/sys.c")
        .file("src/time.c");
    if pthread_enabled {
        build.file("src/pthread.c").flag_if_supported("-pthread");
    }
    build.compile("lunacy_c");
}
