use std::fmt;


#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Continuation<Frame>(Vec<Frame>);

impl<Frame> Continuation<Frame> {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn top_mut(&mut self) -> Option<&mut Frame> {
        let len = self.0.len();
        if len == 0 {
            return None;
        }
        Some(&mut self.0[len - 1])
    }

    pub fn push(&mut self, frame: Frame) {
        self.0.push(frame);
    }

    pub fn pop(&mut self) -> Option<Frame> {
        self.0.pop()
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
