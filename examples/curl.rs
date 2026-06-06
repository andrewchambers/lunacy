#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use lunacy::{curl, fd};

lunacy::entry!(main);

fn main() -> lunacy::ffi::c_int {
    match run() {
        Ok(()) => 0,
        Err(()) => 1,
    }
}

fn run() -> core::result::Result<(), ()> {
    unsafe {
        curl::global_init().map_err(|_| ())?;
    }

    let mut body = Vec::new();
    {
        let mut easy = curl::Easy::new().map_err(|_| ())?;
        easy.set_url(c"https://example.com/").map_err(|_| ())?;
        easy.set_follow_location(true).map_err(|_| ())?;
        easy.set_write_function(|bytes| {
            if body.try_reserve(bytes.len()).is_err() {
                return curl::WriteAction::Abort;
            }

            body.extend_from_slice(bytes);
            curl::WriteAction::Continue
        })
        .map_err(|_| ())?;
        easy.perform().map_err(|_| ())?;
    }

    fd::write_all(fd::STDOUT, &body).map_err(|_| ())
}
