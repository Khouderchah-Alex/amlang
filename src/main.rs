use std::fs::File;
use std::io::{self, BufReader};

mod builtin;
mod cons_list;
mod environment;
mod function;
mod interpreter;
mod parser;
mod sexp;
mod tokenizer;

fn main() -> io::Result<()> {
    let f = File::open("test.aml")?;
    let result = tokenizer::tokenize(BufReader::new(f)).unwrap();
    let sexps = parser::parse(result).unwrap();

    // Basic REPL over forms.
    for sexp in &sexps {
        println!("> {:#}", sexp);
        let result = interpreter::eval(sexp);
        match result {
            Ok(val) => {
                println!("-> {:#}", val);
            }
            Err(err) => {
                println!("-> {}", err);
            }
        }
        println!();
    }

    Ok(())
}
