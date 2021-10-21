mod common;

use std::convert::TryFrom;

use amlang::agent::amlang_agent::RunError;
use amlang::agent::Agent;
use amlang::primitive::error::ErrKind;
use amlang::primitive::{Node, Number, Primitive};


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
fn lambda_param_node_body() {
    let mut lang_agent = common::setup().unwrap();

    // Requires concretization to work properly to avoid returning the abstract
    // param node itself.
    let results = common::results(&mut lang_agent, "((lambda (a) a) 4)");
    assert_eq!(results, vec![Number::Integer(4).into()]);
}

#[test]
fn lambda_single_body() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((lambda (a) (+ a a)) 4)");
    assert_eq!(results, vec![Number::Integer(8).into()]);
}

#[test]
fn lambda_seq_body() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((lambda (a) (jump a) (curr)) lambda)");
    assert_eq!(
        results,
        vec![
            Node::new(
                lang_agent.state().context().lang_env(),
                lang_agent.state().context().lambda
            )
            .into()
        ]
    );
}

#[test]
fn lambda_branch_body() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(def a (lambda (e) (if (eq e 1) 0 (+ e 2))))
         (a 1)
         (a 2)",
    );
    assert_eq!(results[1], Number::Integer(0).into());
    assert_eq!(results[2], Number::Integer(4).into());
}

#[test]
fn lambda_nested_exec() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((lambda (a) (+ a a)) (* 4 2))");
    assert_eq!(results, vec![Number::Integer(16).into()]);
}

#[test]
fn lambda_proc() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((lambda (a b) (a b 4)) + 40)");
    assert_eq!(results, vec![Number::Integer(44).into()]);
}

#[test]
fn lambda_duplicate_argname() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results_with_errors(&mut lang_agent, "(lambda (a a) (+ a a))");
    if let Err(RunError::CompileError(err)) = &results[0] {
        assert!(matches!(err.kind(), ErrKind::InvalidArgument { .. }));
    } else {
        panic!();
    }
}

#[test]
fn let_basic() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(let ((a 2)
               (b 4))
           (+ a b))",
    );
    assert_eq!(results, vec![Number::Integer(6).into()]);
}

#[test]
fn let_star() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(let* ((a 2)
                (b a)
                (c b))
           (+ a b c))",
    );
    assert_eq!(results, vec![Number::Integer(6).into()]);
}

#[test]
fn basic_apply() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(apply + '(1 2))");
    assert_eq!(results, vec![Number::Integer(3).into()]);
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

#[test]
fn def_recursive_lambda() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(def fact (lambda (n)
           (if (eq n 1) 1
             (* n (fact (- n 1))))))

         (fact 4)",
    );
    assert_eq!(results[1], Number::Integer(24).into());
}

#[test]
fn reify_apply() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(car
           (eval  ;; Reify.
             (eval '(+ 1 2))))  ;; Create Procedure::Application.",
    );
    assert_eq!(
        results[0],
        Node::new(
            lang_agent.state().context().lang_env(),
            lang_agent.state().context().apply
        )
        .into()
    );
}

#[test]
fn improper_list() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(eq '(1 2 . 3) (cons 1 (cons 2 3)))");
    assert_eq!(
        results[0],
        Node::new(
            lang_agent.state().context().lang_env(),
            lang_agent.state().context().t
        )
        .into()
    );
}

#[test]
fn basic_ask_tell() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(jump lambda)
         (ask lambda _ _)
         (def VariantOf)
         (def Procedure)
         (tell lambda VariantOf Procedure)
         (ask lambda _ _)
         (ask _ VariantOf _)",
    );

    assert_eq!(results[1].iter().count(), 1);
    assert_eq!(results[5].iter().count(), 2);
    assert_eq!(results[6].iter().count(), 1);
}

#[test]
fn jump_import() {
    let mut lang_agent = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(jump (import lambda))
         (eq (curr) (import lambda))",
    );

    assert_eq!(
        results[1],
        Node::new(
            lang_agent.state().context().lang_env(),
            lang_agent.state().context().t
        )
        .into()
    );
}
