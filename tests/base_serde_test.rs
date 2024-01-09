mod common;

use amlang::agent::{BaseDeserializer, BaseSerializer};
use amlang::prelude::*;
use serde::{Deserialize, Serialize};


#[test]
fn test_struct() {
    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Test {
        int: u32,
        seq: Vec<String>,
    }

    let (mut agent, _manager) = common::setup().unwrap();
    let original = Test {
        int: 1,
        seq: vec!["a".to_owned(), "b".to_owned()],
    };

    // TODO(func) Have list! support (a . b)
    let expected = list!(
        "Test".to_symbol_or_panic(policy_base),
        Cons::new("int".to_symbol_or_panic(policy_base), Some(1u32.into())),
        Cons::new(
            "seq".to_symbol_or_panic(policy_base),
            list!("a".to_string(), "b".to_string())
        )
    );

    let serialized = BaseSerializer::to_sexp(&mut agent, &original).unwrap();
    assert_eq!(expected, *serialized);

    let wrong_int_type = list!(
        "Test".to_symbol_or_panic(policy_base),
        Cons::new("int".to_symbol_or_panic(policy_base), Some(1u8.into())),
        Cons::new(
            "seq".to_symbol_or_panic(policy_base),
            list!("a".to_string(), "b".to_string())
        )
    );
    assert_ne!(wrong_int_type, *serialized);

    let deserialized = Test::deserialize(&mut BaseDeserializer::from_sexp(*serialized)).unwrap();
    assert_eq!(original, deserialized);
}
