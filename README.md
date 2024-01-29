![alt text](https://github.com/Khouderchah-Alex/amlang/blob/master/logo/logo.png "Amlang")

# Summary
The key skill required to develop effective software is managing abstractions
(designing, adapting, nesting, and contextualizing them). If language systems
exist to facilitate building effective software, then the defining
characteristic of a next-generation language system is one which aids in the
managing of abstractions. Amlang is a platform designed to do this.

Managing abstractions goes beyond developing software. One could allow for a
theory of mind which posits that the mind is abstractions of different forms and
modalities, all the way down. Permitting that line of thought, managing
abstractions is the key matter of AI broadly and AGI in particular. Amlang is a
platform designed to facilitate software which seeks to do this.

Amlang is an intersection of programming language, database, and simulation
system. Both a language system in its own right, and a library to build language
systems out of. A key principle of Amlang is the ability to both collapse and
reproduce the context of data.

# Key Abstractions
## Core
  - Environment: A [triple store](https://en.wikipedia.org/wiki/Triplestore)
    capable of representing triples as nodes. Environments can store S-exps, but
    can also be represented to varying degrees as S-exps. Environments are owned
    by the MetaEnvironment, which treats them as nodes that can be connected and
    related as any other node.
  - S-exp: Basically an [N-tree](https://en.wikipedia.org/wiki/M-ary_tree)
    composed of primitive types (importantly, including nodes of Environments)
    and a generic glue called Cons.

Concepts could be initially represented as nodes in a graph and queried through
the (relatively) slow mechanims of a graph-db/triple-store. As usage hardens
into particular forms, S-exps can be used to represent the relevant relations
and bypass the use of graph queries. We might think of this as compiling.
Indeed, we can always "compile" down an entire Environment into a set of N-trees
(read: S-exps) if we don't need generic querying capabilities. We might think of
this as a way of modeling the Environment to be something other than a
triple-store.

In the other direction, we could begin with a monolithic S-exp (or an external
interface/system) and incrementally abstract out relevant concepts into the
Environment. We can do this more than once, creating different models of one
structure and pitting them against each other or using them in different
contexts. We might think of this as an Environment modeling a set of S-exps.

The duality between Environment and S-exp forms what we call a **structured
metagraph**, a single structure capable of collapsing and reproducing parts of
itself.

Finally, it's worth noting that an S-exp is a stone's throw from a array. This
flattening could be accomplished from anywhere between trivial serialization to
full-on compilation, but allows for structured metagraphs to be "compiled" down
to native formats independent of this project. When we support the text/binary
-> S-exp direction as well, the knot tied between Environment and S-exp extends
to text & binaries and forms a single structure of computing.
  
## Agent
  - Agent: Agents exist in Environments, and use Interpreters to act on S-exps
    (which can come from Environments, other Agents, or external entities like
    human users).
  - Interpreter: Interpreters define Agent actions given S-exps. Alternatively,
    one could look at an Interpreter as a context of meaning for S-exps (which
    are otherwise inert N-trees).
  - Context: Contexts are a link between the Rust world and Amlang world. They
    allow for Rust code to talk about specific Nodes as variants of an enum
    (particularly useful when writing Interpreters), and for Amlang to use Nodes
    to talk about Rust interfaces. Contexts are a key part of getting reflection
    b/w Rust and Amlang, both for clients of this library building custom
    functionality and for this library itself to expose the implementation of
    Amlang to the interface. Amlang doesn't require reflection, but certain
    self-modifying projects do.

## Further reading
The concepts behind the project are further described in this article series:
  - [Part 0: Background](https://alexkhouderchah.com/articles/ai/amlang_0.html)
  - [Part 1: Environment](https://alexkhouderchah.com/articles/ai/amlang_1.html)
  - Part 2: Agents (WIP)
  - Part 3: Operational Design (WIP)

# Demo REPL

`cargo run --example amlang_repl` will run a REPL with the built-in `AmlangInterpreter`.

This REPL is essentially a dynamic, interpreted version of what would
otherwise be directly using the `Agent` API in Rust.

The `AmlangInterpreter` provides a model of the core `Agent` API and some basic
Lisp-like computational primitives, but exposed at the `Agent`-level rather than
the Rust-API-level. `Agents` can use any `Interpreter`, and at this stage in
maturity, downstream usage will likely involve custom `Interpreters` manually
written in Rust. However, the `AmlangInterpreter` is still useful for quick
prototyping and `Environment` modifications, and a long-term goal is to be able to
collapse `AmlangInterpreter` code into Rust and follow rustc down to machine code.

The `AmlangInterpreter` is currently missing several key interfaces available in
Rust, but will be coming soon.

Basic commands:
  - Unnamed Nodes:
    - `(anon)` - Create atomic Node in current Env
    - `(anon (+ 1 2))` - Create structured Node in current Env, containing the interpretation of the associated structure (with the amlang interpreter in this REPL)
    - `(set! $node $structure)` - Set the structure of a Node
    - `$node` - Return the structure of a Node (which may itself contain Nodes), or the Node again if atomic
  - Named Nodes
    - `(def name)` - Like `(node)`, but naming the Node
    - `(def name (+ 1 2))` - Like `(node (+ 1 2))`, but naming the Node
    - `(set! name structure)` - Set the structure of a Node
    - `name` - Return the structure of a Node (which may itself contain Nodes), or the Node again if atomic
  - Triples
    - `(tell a b c)` - Create a triple using the Nodes a b c
    - `(ask _ b _)` - Return a list of all the triples with b as the predicate (the _ can go anywhere)
  - Agent Movement
    - `(jump a)` - Jump to a (note that this may change envs)
    - `(curr)` - Get current Node location
  - Lisp-like Computational Structure:
    - `(lambda (args) body)`
    - `(let ((name1 val1) (name2 val2)) body-using-names)`
    - `(quote sexp)` or `'sexp`
    - `(car sexp)`, `(cdr sexp)`, `(cons sexp)`
    - `(println sexp)`

Note that the Environments you work with will be serialized and the
changes available the next time you run the REPL. Run `cargo run
--example simple_repl -- -r` to reset the saved state before running
the REPL.

The [lang_test](tests/lang_test.rs) contains examples of guaranteed-supported behavior.
