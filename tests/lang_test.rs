mod common;

use amlang::primitive::Number;


#[test]
fn basic_arithmetic() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(+ 1 2) (+ 2 2)");
    assert_eq!(
        results,
        vec![Number::Integer(3).into(), Number::Integer(4).into()]
    );

    let results = common::results(
        &mut lang_agent,
        "(* (+ 1 1) 3)
         (* (+ 1 1) 3.)",
    );
    assert_eq!(
        results,
        vec![Number::Integer(6).into(), Number::Float(6.0).into()]
    );

    let results = common::results(
        &mut lang_agent,
        "(/ (- 1 1) 2)
         (/ (+ 1 1) 2)",
    );
    assert_eq!(
        results,
        vec![Number::Float(0.).into(), Number::Float(1.).into()]
    );
}

#[test]
fn basic_lambda() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((lambda (a) (+ a a)) 4)");
    assert_eq!(results, vec![Number::Integer(8).into()]);
}

#[test]
fn lambda_nested_exec() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((lambda (a) (+ a a)) (* 4 2))");
    assert_eq!(results, vec![Number::Integer(16).into()]);
}
