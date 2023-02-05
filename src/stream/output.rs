use std::collections::VecDeque;
use std::io::Write;

use super::Transform;
use crate::error::Error;


pub struct Writer<W: Write> {
    writer: W,
    cache: VecDeque<u8>,
}

impl<W: Write> Writer<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            cache: Default::default(),
        }
    }
}

impl<W: Write> Transform<String, usize> for Writer<W> {
    fn input(&mut self, input: Result<String, Error>) -> Result<bool, Error> {
        let s = input?;
        self.cache.write_all(s.as_bytes()).unwrap();
        Ok(s.len() > 0)
    }

    fn output(&mut self) -> Option<Result<usize, Error>> {
        if self.cache.len() == 0 {
            return None;
        }

        let consumed = self.writer.write(self.cache.as_slices().0).unwrap();
        self.cache.drain(..consumed);
        Some(Ok(consumed))
    }
}
