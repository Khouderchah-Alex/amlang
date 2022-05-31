use lazy_static::lazy_static;
use regex::Regex;

use super::symbol::SymbolError;
use crate::environment::local_node::{LocalId, LocalNode};


/// De-facto specification of amlang identifier format.
fn is_amlang_identifier(s: &str) -> bool {
    match s {
        "+" | "-" | "*" | "/" => true,
        _ if s
            .chars()
            .all(|c| c.is_alphabetic() || c == '_' || c == '-' || c == '*' || c == '!') =>
        {
            true
        }
        _ => false,
    }
}

/// Only accepts non-dunder identifiers.
pub fn policy_base(s: &str) -> Result<(), SymbolError> {
    if is_amlang_identifier(s) {
        if s.chars().take(2).collect::<String>() == "__" {
            Err(SymbolError::DunderPrefix(s.to_string()))
        } else {
            Ok(())
        }
    } else {
        Err(SymbolError::InvalidIdentifier(s.to_string()))
    }
}


pub enum AdminSymbolInfo {
    Identifier,
    LocalNode(LocalNode),
    LocalTriple(usize),
    GlobalNode(LocalNode, LocalNode),
    GlobalTriple(LocalNode, usize),
}

/// Accepts all identifiers.
pub fn policy_admin(s: &str) -> Result<AdminSymbolInfo, SymbolError> {
    if is_amlang_identifier(s) {
        Ok(AdminSymbolInfo::Identifier)
    } else {
        Err(SymbolError::InvalidIdentifier(s.to_string()))
    }
}

/// Accepts all identifiers, as well as local & global nodes/triples.
pub fn policy_env_serde(s: &str) -> Result<AdminSymbolInfo, SymbolError> {
    match s {
        _ if is_amlang_identifier(s) => Ok(AdminSymbolInfo::Identifier),
        _ if s
            .chars()
            .all(|c| c.is_ascii_digit() || c == '^' || c == 't') =>
        {
            lazy_static! {
                static ref LNODE: Regex = Regex::new(r"^\^(\d+)$").unwrap();
                static ref LTRIPLE: Regex = Regex::new(r"^\^t(\d+)$").unwrap();
                static ref GNODE: Regex = Regex::new(r"^\^(\d+)\^(\d+)$").unwrap();
                static ref GTRIPLE: Regex = Regex::new(r"^\^(\d+)\^t(\d+)$").unwrap();
            }

            if let Some(cap) = LNODE.captures(s) {
                Ok(AdminSymbolInfo::LocalNode(LocalNode::new(
                    cap.get(1).unwrap().as_str().parse::<LocalId>().unwrap(),
                )))
            } else if let Some(cap) = LTRIPLE.captures(s) {
                Ok(AdminSymbolInfo::LocalTriple(
                    cap.get(1).unwrap().as_str().parse::<usize>().unwrap(),
                ))
            } else if let Some(cap) = GNODE.captures(s) {
                Ok(AdminSymbolInfo::GlobalNode(
                    LocalNode::new(cap.get(1).unwrap().as_str().parse::<LocalId>().unwrap()),
                    LocalNode::new(cap.get(2).unwrap().as_str().parse::<LocalId>().unwrap()),
                ))
            } else if let Some(cap) = GTRIPLE.captures(s) {
                Ok(AdminSymbolInfo::GlobalTriple(
                    LocalNode::new(cap.get(1).unwrap().as_str().parse::<LocalId>().unwrap()),
                    cap.get(2).unwrap().as_str().parse::<usize>().unwrap(),
                ))
            } else {
                Err(SymbolError::InvalidNodeSpec(s.to_string()))
            }
        }
        _ => Err(SymbolError::InvalidIdentifier(s.to_string())),
    }
}
