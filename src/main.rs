use std::env;
use std::path::Path;

mod builtin;
mod cons_list;
mod environment;
mod function;
mod interpreter;
mod number;
mod parser;
mod sexp;
mod token;

fn usage(args: &Vec<String>) {
    println!(
        "usage: {} [SRC_FILE]",
        Path::new(&args[0]).file_name().unwrap().to_string_lossy()
    );
    println!();
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    return match args.len() {
        1 => Ok(interactive_repl()),
        2 => file_repl(&args[1]),
        n => {
            usage(&args);
            Err(format!("Wrong argument count: {}, expected 0 or 1", n - 1))
        }
    };
}

fn interactive_repl() {
    let stream = token::interactive_stream::InteractiveStream::new();
    let mut peekable = stream.peekable();

    while let Some(sexp) = parser::parse_sexp(&mut peekable, 0).unwrap() {
        let result = interpreter::eval(&sexp);
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
}

fn file_repl(path: &str) -> Result<(), String> {
    let stream = match token::file_stream::FileStream::new(path) {
        Ok(f) => f,
        Err(err) => return Err(format!("{}", err)),
    };
    let sexps = parser::parse(stream).unwrap();

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
