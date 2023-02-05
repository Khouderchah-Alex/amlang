use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use crate::error::Error;


pub struct FileReader {
    reader: Option<BufReader<File>>,
}

impl FileReader {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        Ok(Self {
            reader: Some(reader),
        })
    }
}

impl Iterator for FileReader {
    type Item = Result<String, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(reader) = &mut self.reader {
            let mut out = String::default();
            if reader.read_line(&mut out).unwrap() == 0 {
                self.reader = None;
                return None;
            }
            return Some(Ok(out));
        }
        None
    }
}


pub struct FifoReader {
    reader: Option<BufReader<File>>,
}

impl FifoReader {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let mut options = OpenOptions::new();
        options.read(true);
        if cfg!(unix) {
            options.custom_flags(libc::O_NONBLOCK);
        }

        let file = options.open(path)?;
        let reader = BufReader::new(file);

        Ok(Self {
            reader: Some(reader),
        })
    }
}

impl Iterator for FifoReader {
    type Item = Result<String, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(reader) = &mut self.reader {
            let mut out = String::default();
            return match reader.read_line(&mut out) {
                Ok(size) => {
                    if size == 0 {
                        None
                    } else {
                        Some(Ok(out))
                    }
                }
                Err(err) => {
                    if err.kind() == ErrorKind::WouldBlock {
                        return None;
                    }
                    panic!("{:?}", err);
                }
            };
        }
        None
    }
}


pub struct StringReader {
    string: Option<String>,
}

impl StringReader {
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self {
            string: Some(s.as_ref().to_owned()),
        }
    }
}

impl Iterator for StringReader {
    type Item = Result<String, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(_s) = &mut self.string {
            let mut original = None;
            std::mem::swap(&mut self.string, &mut original);
            return Some(Ok(original.unwrap()));
        }
        None
    }
}