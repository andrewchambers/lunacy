# lunacy

Lunacy is an alternative standard library for rust that is designed
with C programmers in mind.

It runs as a nostd rust environment and provides C like alternatives to the 'normal'
way of doing things in rust.

## Project goals


- Remove layers of abstraction between rust and the OS interfaces.
- Add convenience and safety to some of the more error prone C like interfaces.
- Produce tiny statically linked binaries.
- Support cosmopolitan libc explicitly.

## A small program

```rust
#![no_std]
#![no_main]

use lunacy::{Errno, args::Args, fs::{self, Mode, OpenFlags}};

fn run(args: Args<'_>) -> Result<usize, Errno> {
    let path = args.get(1).unwrap_or(c"README.md");
    let file = fs::open(path, OpenFlags::rdonly(), Mode::empty())?;
    let info = fs::fstat(file.as_fd())?;
    lunacy::println!("{}: {} bytes", path.to_string_lossy(), info.st_size)
}

fn main(args: Args<'_>) -> i32 {
    match run(args) {
        Ok(_) => 0,
        Err(error) => {
            let _ = lunacy::eprintln!("file_info: {error:?}");
            1
        }
    }
}

lunacy::lunacy_main!(main);
```

`file` owns the descriptor and closes it on drop; `as_fd()` borrows it.
Printing returns a byte count or error; a short write is still `Ok(n)`.

`lunacy_main!` supplies the libc allocator, C entry point, and an aborting panic
handler. Set `panic = "abort"` in your application's Cargo profiles; see the
[build guide](docs/building.md). Regular `std` programs can use lunacy without
the macro.

## Try it

With Rust 1.85+, a C11 compiler, and POSIX libc/pthread development files:

```sh
cargo run --release --example file_info -- README.md
cargo run --release --example list -- .
cargo run --release --example threading
```

## More

- [Examples](examples/): [hello](examples/hello.rs), [file metadata](examples/file_info.rs),
  [arguments and directories](examples/list.rs),
  [pipes and environment](examples/pipe.rs), [sockets and readiness](examples/socket.rs),
  [threads and mutexes](examples/threading.rs), [clocks and sleep](examples/time.rs).
- [API source and documentation](src/lib.rs): run `cargo doc --no-deps --open`
  to browse the API locally.
- [Building, static linking, and development checks](docs/building.md).
