# Processes

Lunacy exposes `fork`, `execv`, `execve`, `execvp`, `waitpid`, and `_exit` at the
crate root, with no optional feature required.

| Function | Program selection | Environment |
| --- | --- | --- |
| `execv` | Explicit path | Inherited |
| `execve` | Explicit path | Supplied `envp` |
| `execvp` | Native PATH search, or explicit path when it contains `/` | Inherited |

These vector interfaces cover the argument-passing needs of C's variadic
`execl`, `execle`, and `execlp`. `execvp` preserves libc's shell fallback for
executable files with an unrecognized format. The wrappers add no retries.

Build argument and environment arrays with `CStrArray::new(&[...])`. It borrows
the C strings and allocates their null-terminated pointer array. Include
`argv[0]` yourself; environment entries are complete `NAME=value` strings.
An empty `envp` array requests an empty environment.

```rust,no_run
use lunacy::{CStrArray, execve};

let argv = CStrArray::new(&[c"sh", c"-c", c"exit 0"]);
let envp = CStrArray::new(&[]);
let error = execve(c"/bin/sh", &argv, &envp).unwrap_err();
// Reached only if exec failed; success replaces the process.
```

Exec functions return `Result<Infallible, Errno>`: success cannot return, while
failure leaves the inputs usable. Successful exec runs no Rust destructors and
preserves libc's descriptor inheritance and close-on-exec behavior.

`fork` is unsafe and returns zero in the child or the child's PID in the parent.
After a multithreaded fork, the child must restrict itself to async-signal-safe
operations until exec or `_exit`. Prepare arrays before forking; avoid formatting,
allocation, deallocation, mutexes, and unwinding in that child. `execve` uses the
prepared arrays directly. `execvp` is not guaranteed async-signal-safe.
See the [signal-safety reference](https://man7.org/linux/man-pages/man7/signal-safety.7.html).

`_exit(code)` terminates without Rust destructors, stdio flushing, or atexit
handlers. Use it on the child path if exec fails. The parent uses `waitpid` to
reap the child and `WaitStatus` to inspect its exit status or signal. `waitpid`
preserves the caller's status on errors and on a zero `WNOHANG` result, and
returns `EINTR` without retrying.

See the [complete fork/exec example](../examples/process.rs):

```sh
cargo run --release --example process
```
