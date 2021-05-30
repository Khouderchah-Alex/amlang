use super::*;

use crate::primitive::ToSymbol;


#[test]
fn vec_into_sexp() {
    let expected = "(test ing)".parse::<Sexp>().unwrap();
    let v = vec!["test".to_symbol_or_panic(), "ing".to_symbol_or_panic()];
    assert_eq!(<Sexp>::from(&v), expected);
    assert_eq!(<Sexp>::from(v), expected);
}
