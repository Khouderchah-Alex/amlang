use crate::env::LocalNode;


/// All environments are created with the following Nodes before all other
/// Nodes. This can be seen as a set of implicitly imported Nodes.
pub enum EnvPrelude {
    SelfEnv,
    Designation,
    TellHandler,
    Reserved3,
    Reserved4,
    Reserved5,
    Reserved6,
    Reserved7,
    Reserved8,
    Reserved9,
}


impl EnvPrelude {
    pub const fn local(&self) -> LocalNode {
        match self {
            Self::SelfEnv => LocalNode::new(0),
            Self::Designation => LocalNode::new(1),
            Self::TellHandler => LocalNode::new(2),
            Self::Reserved3 => LocalNode::new(3),
            Self::Reserved4 => LocalNode::new(4),
            Self::Reserved5 => LocalNode::new(5),
            Self::Reserved6 => LocalNode::new(6),
            Self::Reserved7 => LocalNode::new(7),
            Self::Reserved8 => LocalNode::new(8),
            Self::Reserved9 => LocalNode::new(9),
        }
    }

    pub const fn name(&self) -> &str {
        match self {
            Self::SelfEnv => "self_env",
            Self::Designation => "amlang_designator",
            Self::TellHandler => "tell_handler",
            Self::Reserved3
            | Self::Reserved4
            | Self::Reserved5
            | Self::Reserved6
            | Self::Reserved7
            | Self::Reserved8
            | Self::Reserved9 => "RESERVED",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "self_env" => Some(Self::SelfEnv),
            "amlang_designator" => Some(Self::Designation),
            "tell_handler" => Some(Self::TellHandler),
            _ => None,
        }
    }
}
