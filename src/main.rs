use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

mod builtin;
mod cons_list;
mod environment;
mod function;
mod interpreter;
mod number;
mod parser;
mod sexp;
mod tokenizer;

fn usage(args: &Vec<String>) {
    println!(
        "usage: {} SRC_FILE",
        Path::new(&args[0]).file_name().unwrap().to_string_lossy()
    );
    println!();
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        usage(&args);
        return Err("Wrong argument count".to_string());
    }

    let file = match File::open(&args[1]) {
        Ok(f) => f,
        Err(err) => return Err(format!("{}", err)),
    };
    let result = tokenizer::tokenize(BufReader::new(file)).unwrap();
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
