use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Lines};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::error::Error;


pub struct FileReader {
    path: PathBuf,
    reader: Lines<BufReader<File>>,
    line: usize,
}

impl FileReader {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            reader: reader.lines(),
            line: 0,
        })
    }

    pub fn seek_line(&mut self, line: usize) -> Result<(), std::io::Error> {
        if self.line > line {
            *self = Self::new(&self.path)?;
        }

        let diff = line - self.line;
        if diff > 0 {
            self.reader.nth(diff - 1);
        }
        self.line = line;
        Ok(())
    }
}

impl Iterator for FileReader {
    type Item = Result<String, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(res) = self.reader.next() {
            Some(res.map_err(|e| e.into()))
        } else {
            None
        }
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
