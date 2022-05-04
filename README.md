# Amlang
An intersection of programming language, database, and simulation system.
Both a language system in its own right, and a library to build language systems out of.

The concepts behind the project are described in this article series:
  - [Part 0: Background](https://alexkhouderchah.com/articles/ai/amlang_0.html)
  - [Part 1: Environment](https://alexkhouderchah.com/articles/ai/amlang_1.html)
  - Part 2: Agents (WIP)
  - Part 3: Operational Design (WIP)

### Simple REPL

To play around with a simple form of the base language system, run `cargo run --example repl`.

Basic commands:
  - Environment:
    - `(def name)` - Create atomic Node with associated symbol "name"
    - `(def name (+ 1 2))` - Create structured Node, containing evaluation of the associated structure
    - `(tell a b c)` - Create a triple using the existing Nodes associated with symbols a b c
    - `(ask _ b _)` - Return a list of all the triples with b as the predicate (the _ can go anywhere)
    - `(jump a)` - Jump to Node a (note that this may change envs)
    - `(curr)` - Get current Node location
  - Language:
    - `(lambda (args) body)`
    - `(println sexp)`
    - `(quote sexp)` or `'sexp`
    - `(let ((name1 val1) (name2 val2)) body-using-names)`

Note that the Environments you work with will be serialized and the changes available the next time you run the REPL. Run `cargo run --example repl -- -r` to reset the saved state before running the REPL.

The [lang_test](tests/lang_test.rs) contains examples of guaranteed-supported behavior.
