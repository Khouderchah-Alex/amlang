use super::designation::Designation;
use crate::builtin::BUILTINS;
use crate::environment::environment::{EnvObject, Environment};
use crate::environment::mem_environment::MemEnvironment;
use crate::environment::meta_environment::{MetaEnvStructure, MetaEnvironment};
use crate::environment::NodeId;
use crate::function::{EvalErr, Ret};
use crate::interpreter;
use crate::parser;
use crate::primitive::Primitive;
use crate::sexp::Sexp;
use crate::token::interactive_stream::InteractiveStream;


pub struct Agent {
    env: NodeId,
    pos: NodeId,

    // TODO(func) Move to central location.
    meta: MetaEnvironment,
}

impl Agent {
    pub fn new() -> Agent {
        let mut meta = MetaEnvironment::new();
        let meta_self = meta.self_node();
        Agent {
            env: meta.insert_structure(MetaEnvStructure::Env(Box::new(MemEnvironment::new()))),
            pos: meta_self,

            meta,
        }
    }

    pub fn pos(&self) -> NodeId {
        self.pos
    }

    pub fn jump(&mut self, node: NodeId) {
        // TODO(sec) Verify.
        self.pos = node;
    }

    // TODO(func) impl
    // pub fn teleport(&mut self, portal: Portal)

    pub fn env(&mut self) -> &mut EnvObject {
        match self.meta.node_structure(self.env).unwrap() {
            MetaEnvStructure::Env(env) => env.as_mut(),
            _ => panic!(),
        }
    }

    // TODO this belongs in the GenericController, which provides base Eval.
    pub fn run(&mut self) -> Result<(), String> {
        let stream = InteractiveStream::new();
        let mut peekable = stream.peekable();

        loop {
            let sexp = match parser::parse_sexp(&mut peekable, 0) {
                Ok(Some(parsed)) => parsed,
                Ok(None) => return Ok(()),
                Err(err) => {
                    println!(" {}", err);
                    println!("");
                    continue;
                }
            };

            let result = interpreter::eval(&sexp, self);
            match result {
                Ok(val) => {
                    println!("-> {}", val);
                }
                Err(err) => {
                    println!(" {}", err);
                }
            }
            println!();
        }
    }
}

impl Designation for Agent {
    fn designate(&mut self, designator: &Primitive) -> Ret {
        return match designator {
            Primitive::Symbol(symbol) => {
                // TODO this should be part of GenericController.
                let value = BUILTINS.lookup(symbol);
                match value {
                    Some(builtin) => Ok(Sexp::Primitive(Primitive::BuiltIn(builtin))),
                    None => Err(EvalErr::UnboundSymbol(symbol.clone())),
                }
            }
            Primitive::Node(node) => Ok(self
                .env()
                .node_structure(*node)
                .cloned()
                .unwrap_or_else(Default::default)),
            // Base case for self-designating.
            _ => Ok(Sexp::Primitive(designator.clone())),
        };
    }
}
