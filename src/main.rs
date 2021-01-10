
use std::fs::File;
use std::io::{BufReader,self};

mod cons_list;
mod parser;
mod sexp;
mod tokenizer;

fn main() -> io::Result<()> {
    let f = File::open("test.aml")?;
    let result = tokenizer::tokenize(BufReader::new(f)).unwrap();
    let cons = parser::parse(result).unwrap();
    println!("{:#}", cons);

    Ok(())
}
