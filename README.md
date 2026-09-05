# lunacy

Lunacy is an alternative standard library for Rust, designed with C programmers
in mind. It works in `no_std` programs and offers C-like access to OS interfaces.

## Project goals

- Remove layers of abstraction between Rust and the OS interfaces.
- Add convenience and safety to error-prone C interfaces.
- Produce tiny statically linked binaries.
- Support cosmopolitan libc explicitly.

## A small program

```toml
[dependencies]
lunacy = "0.1"
```

```rust
#![no_std]
#![no_main]

use lunacy::{Args, Mode, OpenFlags, fstat, open, print};

#[lunacy::main]
fn main(args: Args<'_>) -> i32 {
    let path = args.get(1).unwrap_or(c"README.md");
    let file = open(path, OpenFlags::rdonly(), Mode::empty()).expect("open failed");
    let info = fstat(file.as_fd()).expect("fstat failed");
    print!("{}: {} bytes\n", path.to_string_lossy(), info.st_size).expect("write failed");
    0
}
```

Set `panic = "abort"` in your application's Cargo profiles; see the
[build guide](docs/building.md). This demo aborts on errors; the file closes on drop.

## Try it

Requires Rust 1.85+, GCC or Clang, and POSIX libc/pthread development files.

```sh
cargo run --release --example file_info -- README.md
cargo run --release --example list -- .
cargo run --release --example threading
```

## Using cargo-cosmopolitan

[cargo-cosmopolitan](https://github.com/andrewchambers/cargo-cosmopolitan) builds
`no_std + alloc` Lunacy applications as Cosmopolitan Actually Portable Executables:
one file containing x86-64 and ARM64 code, linked to Cosmopolitan libc. The
`#[lunacy::main]` entry point shown above supplies the runtime, allocator, and
panic handler.

Follow its [installation instructions](https://github.com/andrewchambers/cargo-cosmopolitan#install)
to set up Cosmopolitan and the pinned Rust nightly with `rust-src` on a Linux
build host. Then, from your application's directory:

```sh
cargo cosmopolitan preflight
cargo cosmopolitan build --release
cargo cosmopolitan run --release -- arguments
```

The executable is written to `target/cosmopolitan/release/<name>.com`.
Use a separate application package depending on Lunacy: this repository's examples
pull in the `libc` development dependency, which does not support the Cosmopolitan
target. Full Rust `std` and native Rust TLS are also unsupported.

## More

- [Examples](examples/), including [fork and exec](examples/process.rs).
- [API source](src/lib.rs), or `cargo doc --no-deps --open`.
- [Building, static linking, and development checks](docs/building.md).
