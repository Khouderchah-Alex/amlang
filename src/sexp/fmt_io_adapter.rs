//! Use a fmt::Write as an io::Write.
//!
//! std has both fmt::Write & io::Write traits. In cases where we want generic
//! IO methods, we impl over io::Write, but fmt::Display uses fmt::Write rather
//! than io::Write. This module allows for io::Write-generic code to be used by,
//! for example, code using fmt::Formatter.

use std::str::from_utf8;
use std::{fmt, io};


pub struct FmtIoAdapter<'a, F: fmt::Write> {
    fmt_writer: &'a mut F,
}

impl<'a, F: fmt::Write> FmtIoAdapter<'a, F> {
    pub fn new(fmt_writer: &'a mut F) -> Self {
        Self { fmt_writer }
    }
}


impl<'a, F: fmt::Write> io::Write for FmtIoAdapter<'a, F> {
    fn write(&mut self, bytes: &[u8]) -> std::result::Result<usize, std::io::Error> {
        // fmt::Write only takes UTF-8, while io::Write is a byte-oriented sink.
        let utf = match from_utf8(bytes) {
            Ok(s) => s,
            Err(err) => return Err(io::Error::new(io::ErrorKind::InvalidData, err)),
        };

        match self.fmt_writer.write_str(utf) {
            Ok(()) => Ok(utf.len()),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }

    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "fmt::Writes are not flushable",
        ))
    }
}
