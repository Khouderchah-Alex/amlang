
use std::fs::File;
use std::io::{BufReader,self};

mod sexp;
mod tokenizer;

fn main() -> io::Result<()> {
    let f = File::open("test.aml")?;
    let result = tokenizer::tokenize(BufReader::new(f)).unwrap();
    println!("{:#?}", result);

    Ok(())
}
