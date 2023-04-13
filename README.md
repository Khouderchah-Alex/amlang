![alt text](https://github.com/Khouderchah-Alex/amlang/blob/master/logo/logo.png "Amlang")

# Summary
An intersection of programming language, database, and simulation system. Both a
language system in its own right, and a library to build language systems out
of. A key principle of Amlang is the ability to both collapse and reproduce the
context of data.

One can think of an `Environment` as a graph database with "wormholes". `Agents`
exist in and interact with their `Environment` and posssibly with each other.
[S-exp](https://en.wikipedia.org/wiki/S-expression)-like `Structures` drive
`Agent` behavior, whether those `Structures` come from the `Environment`, from
other `Agents`, from human users, or even from themselves. But structure alone
has no meaning; `Agents` use `Interpreters` to tie `Structures` to actual
behavior.

`Agents` can also use local state to form "overlays" on top of their
`Environment`, allowing different `Agents` to perceive the same `Environment`
differently. Those "overlays", represented as `Structures`, can also be stored
in the `Environment`, examined by `Agents`, and shared or built upon. More
broadly, Amlang has the capability to deeply
[reify](https://en.wikipedia.org/wiki/Reification_(computer_science)) itself,
although it isn't by default.

The concepts behind the project are further described in this article series:
  - [Part 0: Background](https://alexkhouderchah.com/articles/ai/amlang_0.html)
  - [Part 1: Environment](https://alexkhouderchah.com/articles/ai/amlang_1.html)
  - Part 2: Agents (WIP)
  - Part 3: Operational Design (WIP)

### AmlangInterpreter REPL

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
    - `(node)` - Create atomic Node in current Env
    - `(node (+ 1 2))` - Create structured Node in current Env, containing the interpretation of the associated structure (with the amlang interpreter in this REPL)
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
