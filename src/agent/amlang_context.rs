use crate::environment::environment::EnvObject;
use crate::environment::LocalNode;


// TODO(perf) Make generic over Environment, but in a manner consistent with
// library usage and without pushing templating over most of the codebase
// (e.g. changing build flag to select Overlay class).
#[derive(Clone, Debug)]
pub struct AmlangContext {
    meta: Box<EnvObject>,

    // Relative to meta env.
    pub lang_env: LocalNode,
    pub imports: LocalNode,
    pub import_table: LocalNode,
    pub serialize_path: LocalNode,

    // All below are relative to lang_env.
    pub quote: LocalNode,
    pub lambda: LocalNode,
    pub fexpr: LocalNode,
    pub def: LocalNode,
    pub tell: LocalNode,
    pub curr: LocalNode,
    pub jump: LocalNode,
    pub ask: LocalNode,
    pub placeholder: LocalNode,
    pub apply: LocalNode,
    pub eval: LocalNode,
    pub exec: LocalNode,
    pub import: LocalNode,
    pub progn: LocalNode,
    pub branch: LocalNode,
    pub let_basic: LocalNode,
    pub let_rec: LocalNode,
    pub env_find: LocalNode,
    pub t: LocalNode,
    pub f: LocalNode,
    pub eq: LocalNode,
    pub symbol_table: LocalNode,
    pub local_node_table: LocalNode,
    pub label: LocalNode,
}

/// All environments are created with the following Nodes before all other
/// Nodes. This can be seen as a set of implicitly imported Nodes.
pub enum EnvPrelude {
    SelfEnv,
    Designation,
    TellHandler,
    Reserved0,
    Reserved1,
    Reserved2,
    Reserved3,
    Reserved4,
    Reserved5,
    Reserved6,
}


impl AmlangContext {
    pub(super) fn new(meta: Box<EnvObject>) -> Self {
        let placeholder = LocalNode::new(1);
        Self {
            meta,

            // This is delicate; putting placeholders here, which must be set
            // properly during bootstrapping.
            lang_env: placeholder.clone(),
            imports: placeholder.clone(),
            import_table: placeholder.clone(),
            serialize_path: placeholder.clone(),

            quote: placeholder.clone(),
            lambda: placeholder.clone(),
            fexpr: placeholder.clone(),
            def: placeholder.clone(),
            tell: placeholder.clone(),
            curr: placeholder.clone(),
            jump: placeholder.clone(),
            ask: placeholder.clone(),
            placeholder: placeholder.clone(),
            apply: placeholder.clone(),
            eval: placeholder.clone(),
            exec: placeholder.clone(),
            import: placeholder.clone(),
            progn: placeholder.clone(),
            branch: placeholder.clone(),
            let_basic: placeholder.clone(),
            let_rec: placeholder.clone(),
            env_find: placeholder.clone(),
            t: placeholder.clone(),
            f: placeholder.clone(),
            eq: placeholder.clone(),
            symbol_table: placeholder.clone(),
            local_node_table: placeholder.clone(),
            label: placeholder.clone(),
        }
    }

    pub fn meta(&self) -> &Box<EnvObject> {
        &self.meta
    }
    pub fn meta_mut(&mut self) -> &mut Box<EnvObject> {
        &mut self.meta
    }

    pub fn lang_env(&self) -> LocalNode {
        self.lang_env
    }

    pub const fn self_node(&self) -> LocalNode {
        EnvPrelude::SelfEnv.local()
    }
    pub const fn designation(&self) -> LocalNode {
        EnvPrelude::Designation.local()
    }
    pub const fn tell_handler(&self) -> LocalNode {
        EnvPrelude::TellHandler.local()
    }
}

impl EnvPrelude {
    pub const fn local(&self) -> LocalNode {
        match self {
            Self::SelfEnv => LocalNode::new(0),
            Self::Designation => LocalNode::new(1),
            Self::TellHandler => LocalNode::new(2),
            Self::Reserved0 => LocalNode::new(3),
            Self::Reserved1 => LocalNode::new(4),
            Self::Reserved2 => LocalNode::new(5),
            Self::Reserved3 => LocalNode::new(6),
            Self::Reserved4 => LocalNode::new(7),
            Self::Reserved5 => LocalNode::new(8),
            Self::Reserved6 => LocalNode::new(9),
        }
    }

    pub const fn name(&self) -> &str {
        match self {
            Self::SelfEnv => "self_env",
            Self::Designation => "amlang_designator",
            Self::TellHandler => "tell_handler",
            Self::Reserved0
            | Self::Reserved1
            | Self::Reserved2
            | Self::Reserved3
            | Self::Reserved4
            | Self::Reserved5
            | Self::Reserved6 => "RESERVED",
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
