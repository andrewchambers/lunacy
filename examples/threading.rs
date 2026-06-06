#![no_std]
#![no_main]

extern crate alloc;

use alloc::{format, sync::Arc, vec::Vec};
use lunacy::{fd, pthread};

lunacy::entry!(main);

fn main() -> lunacy::Result<()> {
    let workers = 4_usize;
    let iterations = 1024_usize;
    let counter = Arc::new(pthread::Mutex::new(0_usize)?);
    let mut threads = Vec::new();

    for _ in 0..workers {
        let counter = Arc::clone(&counter);
        threads.push(pthread::spawn(move || {
            for _ in 0..iterations {
                let mut value = counter.lock().unwrap();
                *value += 1;
            }
        })?);
    }

    for thread in threads {
        thread.join()?;
    }

    let total = *counter.lock()?;
    let message = format!("workers={workers} iterations={iterations} total={total}\n");

    fd::write_all(fd::STDOUT, message.as_bytes())
}
