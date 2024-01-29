mod common;

use amlang::prelude::*;
use serde::{Deserialize, Serialize};


#[test]
fn test_struct() {
    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Test {
        int: u32,
        seq: Vec<String>,
        b: bool,
    }

    let (mut agent, _manager) = common::setup().unwrap();
    let original = Test {
        int: 1,
        seq: vec!["a".to_owned(), "b".to_owned()],
        b: true,
    };

    // TODO(func) Have list! support (a . b)
    let expected = list!(
        "Test".to_symbol_or_panic(policy_base),
        Cons::new("int".to_symbol_or_panic(policy_base), Some(1u32.into())),
        Cons::new(
            "seq".to_symbol_or_panic(policy_base),
            list!("a".to_string(), "b".to_string())
        ),
        Cons::new(
            "b".to_symbol_or_panic(policy_base),
            amlang_node!(t, agent.context()),
        )
    );

    let serialized = agent.reify(&original).unwrap();
    println!("{}", serialized);
    assert_eq!(expected, *serialized);

    let wrong_int_type = list!(
        "Test".to_symbol_or_panic(policy_base),
        Cons::new("int".to_symbol_or_panic(policy_base), Some(1u8.into())),
        Cons::new(
            "seq".to_symbol_or_panic(policy_base),
            list!("a".to_string(), "b".to_string())
        ),
        Cons::new(
            "b".to_symbol_or_panic(policy_base),
            amlang_node!(t, agent.context()),
        )
    );
    assert_ne!(wrong_int_type, *serialized);

    let deserialized = agent.reflect::<Test>(*serialized).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn test_enum() {
    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Sub {
        i: i32,
        am: u64,
        groot: i8,
    }

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    enum Test {
        Unit,
        Int(i8),
        Seq(Vec<f64>),
        Tuple(u16, i32),
        Struct(Sub),
    }

    let (mut agent, _manager) = common::setup().unwrap();

    let unit = Test::Unit;
    let expected: Sexp = "Unit".parse().unwrap();
    let serialized = agent.reify(&unit).unwrap();
    assert_eq!(expected, *serialized);
    let deserialized = agent.reflect::<Test>(*serialized).unwrap();
    assert_eq!(unit, deserialized);

    let int = Test::Int(42);
    let expected: Sexp = "(Int 42)".parse().unwrap();
    let serialized = agent.reify(&int).unwrap();
    assert_eq!(expected, *serialized);
    let deserialized = agent.reflect::<Test>(*serialized).unwrap();
    assert_eq!(int, deserialized);

    let seq = Test::Seq(vec![4., 2.]);
    let expected: Sexp = "(Seq (4. 2.))".parse().unwrap();
    let serialized = agent.reify(&seq).unwrap();
    assert_eq!(expected, *serialized);
    let deserialized = agent.reflect::<Test>(*serialized).unwrap();
    assert_eq!(seq, deserialized);

    let tuple = Test::Tuple(4, 2);
    let expected: Sexp = "(Tuple 4 2)".parse().unwrap();
    let serialized = agent.reify(&tuple).unwrap();
    assert_eq!(expected, *serialized);
    let deserialized = agent.reflect::<Test>(*serialized).unwrap();
    assert_eq!(tuple, deserialized);

    let sub = Sub {
        i: 0,
        am: 2,
        groot: 4,
    };
    let struct_ = Test::Struct(sub);
    let expected: Sexp = "(Struct (Sub (i . 0) (am . 2) (groot . 4)))"
        .parse()
        .unwrap();
    let serialized = agent.reify(&struct_).unwrap();
    assert_eq!(expected, *serialized);
    let deserialized = agent.reflect::<Test>(*serialized).unwrap();
    assert_eq!(struct_, deserialized);
}
