mod common;

use std::convert::TryFrom;

use amlang::agent::amlang_agent::RunError;
use amlang::agent::Agent;
use amlang::lang_err::{ErrKind, LangErr};
use amlang::primitive::{Number, Primitive};


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

#[test]
fn lambda_duplicate_argname() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results_with_errors(&mut lang_agent, "(lambda (a a) (+ a a))");
    assert!(matches!(
        results[0],
        Err(RunError::CompileError(LangErr {
            kind: ErrKind::InvalidArgument { .. },
            ..
        }))
    ));
}

#[test]
fn basic_fexpr() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((fexpr (a) (car (cdr a))) (+ 1 2))");
    assert_eq!(results, vec![Number::Integer(1).into()]);
}

#[test]
fn def_atom() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(def a)");
    // Atom should designate to itself.
    assert_eq!(
        lang_agent
            .state_mut()
            .designate(Primitive::try_from(results[0].clone()).unwrap())
            .unwrap(),
        results[0],
    );
}

#[test]
fn def_number() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(def a 2)");
    assert_eq!(
        lang_agent
            .state_mut()
            .designate(Primitive::try_from(results[0].clone()).unwrap())
            .unwrap(),
        Number::Integer(2).into()
    );
}

#[test]
fn def_lambda() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(def a (lambda (e) (+ e 2))) (a 2)");
    assert_eq!(results[1], Number::Integer(4).into());
}
