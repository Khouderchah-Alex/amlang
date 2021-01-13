use std::fs::File;
use std::io::{self, BufReader};

mod cons_list;
mod environment;
mod parser;
mod sexp;
mod tokenizer;

fn main() -> io::Result<()> {
    let f = File::open("test.aml")?;
    let result = tokenizer::tokenize(BufReader::new(f)).unwrap();
    let sexps = parser::parse(result).unwrap();
    for sexp in sexps {
        println!("{:#}", sexp);
    }

    Ok(())
}
