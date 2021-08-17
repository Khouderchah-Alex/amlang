use log::debug;
use std::io::{stdout, BufWriter};

use super::agent_state::AgentState;
use crate::agent::exec_state::ExecState;
use crate::model::{Eval, Model};
use crate::primitive::{Node, Primitive};
use crate::sexp::Sexp;


pub trait Agent: Eval {
    fn state(&self) -> &AgentState;
    fn state_mut(&mut self) -> &mut AgentState;
    fn cont(&self) -> &ExecState;
    fn cont_mut(&mut self) -> &mut ExecState;

    fn concretize(&self, node: Node) -> Node {
        for frame in self.cont().iter() {
            if let Some(n) = frame.lookup(node) {
                debug!("concretizing: {} -> {}", node, n);
                return n;
            }
        }
        node
    }

    fn print_list(&mut self, structure: &Sexp) {
        let mut writer = BufWriter::new(stdout());
        if let Err(err) = self.write_list_internal(&mut writer, structure, 0) {
            println!("print_list error: {:?}", err);
        }
    }


    // "Private" methods below. //

    fn write_list_internal<W: std::io::Write>(
        &mut self,
        w: &mut W,
        structure: &Sexp,
        depth: usize,
    ) -> std::io::Result<()> {
        structure.write_list(w, depth, &mut |writer, primitive, depth| {
            self.write_primitive(writer, primitive, depth)
        })
    }

    fn write_primitive<W: std::io::Write>(
        &mut self,
        w: &mut W,
        primitive: &Primitive,
        depth: usize,
    ) -> std::io::Result<()> {
        const MAX_DEPTH: usize = 16;

        match primitive {
            Primitive::Node(raw_node) => {
                let node = self.concretize(*raw_node);
                // Print Nodes as their designators if possible.
                if let Some(sym) = self.state_mut().node_designator(node) {
                    write!(w, "{}", sym.as_str())
                } else if let Some(triple) = self
                    .state_mut()
                    .access_env(node.env())
                    .unwrap()
                    .node_as_triple(node.local())
                {
                    let s = triple.generate_structure(self.state_mut());
                    self.write_list_internal(w, &s, depth + 1)
                } else {
                    let s = if let Some(structure) = self
                        .state_mut()
                        .access_env(node.env())
                        .unwrap()
                        .node_structure(node.local())
                    {
                        write!(w, "{}->", node)?;
                        // Subtle: Cloning of Env doesn't actually copy data. In
                        // this case, the resulting Env object will be invalid
                        // and should only stand as a placeholder to determine
                        // typing.
                        //
                        // TODO(func) SharedEnv impl.
                        structure.clone()
                    } else {
                        return write!(w, "{}", node);
                    };

                    // If we recurse unconditionally, cycles will cause stack
                    // overflows.
                    if s == node.into() || depth > MAX_DEPTH {
                        write!(w, "{}", node)
                    } else {
                        self.write_list_internal(w, &s, depth + 1)
                    }
                }
            }
            Primitive::Procedure(procedure) => {
                let s = procedure.generate_structure(self.state_mut());
                self.write_list_internal(w, &s, depth + 1)
            }
            _ => write!(w, "{}", primitive),
        }
    }
}
