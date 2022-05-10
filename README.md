![alt text](https://github.com/Khouderchah-Alex/amlang/blob/master/logo/logo.png "Amlang")

# Summary
An intersection of programming language, database, and simulation system.
Both a language system in its own right, and a library to build language systems out of.

A 10,000 foot view of Amlang is as a graph database ([triplestore](https://en.wikipedia.org/wiki/Triplestore)) with nodes containing and interconnected by S-exp-like structures, called the Environment. Agents exist in, modify, and explore the Environment, while using local state to apply subjective "lenses" to the shared Environment. Amlang code is evaluated into a representation within the Environment and can itself be explored and modified; consider an Agent exploring the code that it's currently executing. Indeed, many important components from the base system in Rust are [reified](https://en.wikipedia.org/wiki/Reification_(computer_science)) in the Environment, allowing Amlang to modify and extend the system it exists on top of.

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
