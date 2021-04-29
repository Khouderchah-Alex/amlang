use crate::environment::environment::{EnvObject, Environment};
use crate::environment::mem_environment::MemEnvironment;
use crate::environment::meta_environment::{MetaEnvStructure, MetaEnvironment};
use crate::environment::{NodeId, TripleId};
use crate::primitive::Primitive;
use crate::sexp::{cons, Sexp};


pub struct EnvState {
    env: NodeId,
    pos: NodeId,

    // TODO(func) Move to central location.
    designation: NodeId,
    meta: MetaEnvironment,
}

const META_DESIGNATION: &str = "__designatedBy";


impl EnvState {
    pub fn new() -> Self {
        let mut meta = MetaEnvironment::new();
        let env = meta.insert_structure(MetaEnvStructure::Env(Box::new(MemEnvironment::new())));

        let env_obj = EnvState::access_env(&mut meta, env);
        let pos = env_obj.self_node();

        let designation =
            env_obj.insert_structure(Sexp::Primitive(Primitive::SymbolTable(Default::default())));
        if let Some(Sexp::Primitive(Primitive::SymbolTable(table))) =
            env_obj.node_structure(designation)
        {
            table.insert(META_DESIGNATION.to_string(), designation);
        } else {
            panic!("Env designation isn't a symbol table");
        }
        env_obj.insert_triple(designation, designation, designation);

        Self {
            env,
            pos,
            designation,
            meta,
        }
    }

    pub fn pos(&self) -> NodeId {
        self.pos
    }

    pub fn designation(&self) -> NodeId {
        self.designation
    }

    pub fn jump(&mut self, node: NodeId) {
        // TODO(sec) Verify.
        self.pos = node;
    }

    // TODO(func) impl
    // pub fn teleport(&mut self, portal: Portal)

    pub fn env(&mut self) -> &mut EnvObject {
        EnvState::access_env(&mut self.meta, self.env)
    }

    pub fn node_designator(&mut self, node: NodeId) -> Option<Box<Sexp>> {
        let designation = self.designation();
        if node == designation {
            return Some(Box::new(Sexp::Primitive(Primitive::Symbol(
                META_DESIGNATION.to_string(),
            ))));
        }

        let env = self.env();
        let names = env.match_but_object(node, designation);
        if let Some(name_node) = names.iter().next() {
            let name = env.triple_object(*name_node);
            return Some(Box::new(env.node_structure(name).cloned().unwrap()));
        }
        None
    }

    pub fn triple_inner_designators(&mut self, triple: TripleId) -> Box<Sexp> {
        let env = self.env();
        let s = env.triple_subject(triple);
        let p = env.triple_predicate(triple);
        let o = env.triple_object(triple);

        let ss = self.node_designator(s);
        let pp = self.node_designator(p);
        let oo = if p == self.designation() {
            cons(
                Some(Box::new(Sexp::Primitive(Primitive::Symbol(
                    "quote".to_string(),
                )))),
                cons(ss.clone(), None),
            )
        } else {
            self.node_designator(o)
        };
        cons(ss, cons(pp, cons(oo, None))).unwrap()
    }

    // TODO(func) Move to same central location as above.
    fn access_env(meta: &mut MetaEnvironment, node: NodeId) -> &mut EnvObject {
        match meta.node_structure(node).unwrap() {
            MetaEnvStructure::Env(env) => env.as_mut(),
            _ => panic!(),
        }
    }
}
