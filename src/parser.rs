//! Module for parsing Amlang tokens into an AST.

use super::cons_list::ConsList;
use super::sexp::{Cons, Value};
use super::tokenizer::{self, Token};

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
    token: tokenizer::TokenInfo,
}

pub fn parse(tokens: tokenizer::Tokens) -> Result<Cons, ParseError> {
    let mut stack = Vec::<(ConsList, tokenizer::TokenInfo)>::new();
    stack.push((
        ConsList::new(),
        tokenizer::TokenInfo {
            token: tokenizer::Token::Comment("ROOT".to_string()),
            line: 0,
        },
    ));

    for token in tokens {
        match token.token {
            Token::LeftParen => {
                if stack.len() >= MAX_LIST_DEPTH {
                    return Err(ParseError {
                        reason: DepthOverflow,
                        token,
                    });
                }
                stack.push((ConsList::new(), token));
            }
            Token::RightParen => {
                if stack.len() <= 1 {
                    return Err(ParseError {
                        reason: UnmatchedClose,
                        token,
                    });
                }
                let (end, _) = stack.pop().unwrap();
                match &mut stack.last_mut() {
                    Some((last, _)) => unsafe {
                        last.append(Value::Cons(*end.release()));
                    },
                    None => {
                        panic!();
                    }
                }
            }
            Token::Atom(atom) => match &mut stack.last_mut() {
                Some((last, _)) => unsafe {
                    last.append(Value::Atom(atom));
                },
                None => {
                    panic!();
                }
            },
            _ => {}
        }
    }

    if stack.len() > 1 {
        return Err(ParseError {
            reason: UnmatchedOpen,
            token: stack.pop().unwrap().1,
        });
    }

    Ok(*stack.pop().unwrap().0.release())
}
