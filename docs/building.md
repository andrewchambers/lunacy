# Building lunacy

Requires Rust 1.85+, a C11 compiler, and POSIX libc/pthread development headers
and libraries. The C shim is compiled by `cc`; there are no runtime Cargo
dependencies. `libc` is a development dependency used by tests.

## Optional threading

The `pthread` feature enables the `pthread` and `sync` modules and is enabled
by default. To omit these APIs and the pthread development requirement, set
`default-features = false` in your dependency declaration, or build with:

```sh
cargo build --release --no-default-features
```

Without this feature, lunacy skips the pthread shim and its compiler/linker
flags. The remaining APIs can still be used by threads created elsewhere.
Use `features = ["pthread"]` to enable threading explicitly when defaults are off.

## Application profiles

Applications using `lunacy_main!` must select aborting panics in their own
Cargo.toml. A dependency's profile settings do not carry over to its consumers.
The repository uses these settings:

```toml
[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
opt-level = "s"
lto = true
codegen-units = 1
strip = true
```

See [Cargo.toml](../Cargo.toml) and the [hello example](../examples/hello.rs).
The macro is optional for applications that provide their own entry point,
allocator, and panic handling.

## Static linking on Linux

`no_std` does not imply static linking. The default Linux GNU target normally
links glibc dynamically. To build a static musl executable, install the Rust
musl target and a matching C toolchain. With `musl-gcc` on PATH:

```sh
rustup target add x86_64-unknown-linux-musl

CC_x86_64_unknown_linux_musl=musl-gcc \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
cargo build --release --target x86_64-unknown-linux-musl --example hello
```

Both variables must name a compiler/linker for the same target and libc. The
`cc` build dependency also accepts target-specific `CFLAGS` and `AR` settings.

The examples have been built and run on x86-64 Linux with static musl.
Measured stripped release sizes were 26,408 bytes for hello, 26,240 for list,
and 30,352 for threading. Dynamic glibc builds were 18,656, 14,392, and 18,488
bytes respectively, excluding the shared libc. Sizes depend on the toolchain
and the functions the program uses; these are examples, not size guarantees.
Cosmopolitan and other platforms have not yet been validated.

## Development checks

```sh
cargo fmt --check
cargo test
cargo test --no-default-features
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
cargo build --release --examples --all-features
cargo doc --no-deps --all-features
```

The tests include native behavior checks and compile-fail ownership examples.
Tests that change the environment, working directory, or signal handlers use
separate executables to isolate process-wide state.
