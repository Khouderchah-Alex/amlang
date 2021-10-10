use log::warn;

use std::fmt;


#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Continuation<Frame>(Vec<Frame>);

impl<Frame> Continuation<Frame> {
    pub fn new(root: Frame) -> Self {
        Self(vec![root])
    }

    pub fn top(&self) -> &Frame {
        let len = self.depth();
        &self.0[len - 1]
    }
    pub fn top_mut(&mut self) -> &mut Frame {
        let len = self.depth();
        &mut self.0[len - 1]
    }

    pub fn push(&mut self, frame: Frame) {
        self.0.push(frame);
    }

    pub fn pop(&mut self) -> Option<Frame> {
        match self.depth() {
            0 => panic!(),
            1 => {
                warn!("Ignoring pop of sole continuation frame");
                None
            }
            _ => self.0.pop(),
        }
    }

    pub fn depth(&self) -> usize {
        self.0.len()
    }

    /// Iterator from most-recent to least-recent frame.
    pub fn iter(&self) -> impl Iterator<Item = &Frame> {
        self.0.iter().rev()
    }
}


impl<Frame> fmt::Display for Continuation<Frame> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Cont depth {}]", self.depth())
    }
}
