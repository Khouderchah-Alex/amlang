use super::*;

use crate::primitive::symbol_policies::policy_base;
use crate::primitive::ToSymbol;


#[test]
fn vec_into_sexp() {
    let expected = "(test ing)".parse().unwrap();
    let v = vec![
        "test".to_symbol_or_panic(policy_base),
        "ing".to_symbol_or_panic(policy_base),
    ];
    assert_eq!(<Sexp>::from(&v), expected);
    assert_eq!(<Sexp>::from(v), expected);
}
