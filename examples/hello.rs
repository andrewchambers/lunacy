#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use lunacy::fd;

lunacy::entry!(main, args);

fn main(args: lunacy::runtime::Args<'_>) -> lunacy::Result<()> {
    let name = args
        .get(1)
        .and_then(|arg| arg.to_str().ok())
        .unwrap_or("world");
    let message = format!("hello {name}\n");

    fd::write_all(fd::STDOUT, message.as_bytes())
}
