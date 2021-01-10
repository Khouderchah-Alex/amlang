//! Module for parsing Amlang tokens into an AST.

use super::cons_list::ConsList;
use super::sexp::{Cons,Value};
use super::tokenizer::{self,Token};

use self::ParseErrorReason::*;

const MAX_LIST_DEPTH: usize = 128;


#[derive(Debug)]
pub enum ParseErrorReason {
    DepthOverflow,
    UnmatchedOpen,
    UnmatchedClose,
}

#[derive(Debug)]
pub struct ParseError {
    reason: ParseErrorReason,
    token: Option<tokenizer::TokenInfo>,
}

pub fn parse(tokens: tokenizer::Tokens) -> Result<Cons, ParseError> {
    let mut stack = Vec::<ConsList>::new();
    stack.push(ConsList::new());

    for token in tokens {
        match token.token {
            Token::LeftParen => {
                if stack.len() >= MAX_LIST_DEPTH {
                    return Err(ParseError{ reason: DepthOverflow, token: Some(token) });
                }
                stack.push(ConsList::new());
            }
            Token::RightParen => {
                if stack.len() <= 1 {
                    return Err(ParseError{ reason: UnmatchedClose, token: Some(token) });
                }
                let end = stack.pop().unwrap();
                match &mut stack.last_mut() {
                    Some(last) => {
                        unsafe {
                            last.append(Value::Cons(*end.release()));
                        }
                    }
                    None => {
                        panic!();
                    }
                }
            }
            Token::Atom(atom) => {
                match &mut stack.last_mut() {
                    Some(last) => {
                        unsafe {
                            last.append(Value::Atom(atom));
                        }
                    }
                    None => {
                        panic!();
                    }
                }
            }
            _ => {}
        }
    }

    if stack.len() > 1 {
        return Err(ParseError{ reason: UnmatchedOpen, token: None })
    }

    Ok(*stack.pop().unwrap().release())
}
