use super::symbol::SymbolError;

pub fn policy_base(s: &str) -> Result<(), SymbolError> {
    match s {
        "+" | "-" | "*" | "/" => {}
        _ => {
            if !s.chars().all(|c| c.is_alphabetic() || c == '_' || c == '-')
                || s.chars().take(2).collect::<String>() == "__"
            {
                return Err(SymbolError::NonAlphabetic(s.to_string()));
            }
        }
    }
    Ok(())
}

pub fn policy_admin(s: &str) -> Result<(), SymbolError> {
    match s {
        "+" | "-" | "*" | "/" => {}
        _ => {
            if !s.chars().all(|c| c.is_alphabetic() || c == '_' || c == '-')
                && !s
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '^' || c == 't')
            {
                return Err(SymbolError::NonAlphabetic(s.to_string()));
            }
        }
    }
    Ok(())
}
