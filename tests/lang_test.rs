mod common;

use std::convert::TryFrom;

use amlang::primitive::{LangString, Node, Number, Primitive};
use amlang::sexp::{Cons, Sexp};
use amlang::{amlang_node, break_sexp};


#[test]
fn basic_arithmetic() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(+ 1 2) (+ 2 2)");
    assert_eq!(results, vec![Number::I64(3).into(), Number::I64(4).into()]);

    let results = common::results(
        &mut lang_agent,
        "(* (+ 1 1) 3)
         (* (+ 1. 1.) 3.)",
    );
    assert_eq!(
        results,
        vec![Number::I64(6).into(), Number::F64(6.0).into()]
    );

    let results = common::results(
        &mut lang_agent,
        "(/ (- 1 1) 2)
         (/ (+ 1 1) 2)",
    );
    assert_eq!(results, vec![Number::I64(0).into(), Number::I64(1).into()]);
}

#[test]
fn lambda_param_node_body() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    // Requires concretization to work properly to avoid returning the abstract
    // param node itself.
    let results = common::results(&mut lang_agent, "((lambda (a) a) 4)");
    assert_eq!(results, vec![Number::I64(4).into()]);
}

#[test]
fn lambda_single_body() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((lambda (a) (+ a a)) 4)");
    assert_eq!(results, vec![Number::I64(8).into()]);
}

#[test]
fn lambda_seq_body() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((lambda (a) (jump a) (curr)) lambda)");
    assert_eq!(
        results,
        vec![amlang_node!(lambda, lang_agent.context()).into()]
    );
}

#[test]
fn lambda_branch_body() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(def a (lambda (e) (if (eq e 1) 0 (+ e 2))))
         (a 1)
         (a 2)",
    );
    assert_eq!(results[1], Number::I64(0).into());
    assert_eq!(results[2], Number::I64(4).into());
}

#[test]
fn lambda_nested_exec() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((lambda (a) (+ a a)) (* 4 2))");
    assert_eq!(results, vec![Number::I64(16).into()]);
}

#[test]
fn lambda_proc() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((lambda (a b) (a b 4)) + 40)");
    assert_eq!(results, vec![Number::I64(44).into()]);
}

#[test]
fn lambda_duplicate_argname() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results_with_errors(&mut lang_agent, "(lambda (a a) (+ a a))");

    let err = results[0].as_ref().unwrap_err().kind().reify();
    let (_, kind, _) = break_sexp!(err => (LangString, LangString; remainder)).unwrap();
    assert_eq!(kind.as_str(), "Invalid argument");
}

#[test]
fn let_basic() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(let ((a 2)
               (b 4))
           (+ a b))",
    );
    assert_eq!(results, vec![Number::I64(6).into()]);
}

#[test]
fn let_rec_vals() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(letrec ((a 2)
                  (b a)
                  (c b))
           (+ a b c))",
    );
    assert_eq!(results, vec![Number::I64(6).into()]);
}

#[test]
fn let_rec_lambdas() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(letrec ((is-even (lambda (n)
                     (if (eq 0 n) true
                       (is-odd (- n 1)))))
                  (is-odd (lambda (n)
                     (if (eq 0 n) false
                       (is-even (- n 1))))))

           ;; Consing since seq consumes intermediate results.
           (cons (is-even 99) (is-odd 33)))",
    );

    let cons = Cons::try_from(results[0].clone()).unwrap();
    assert_eq!(
        *cons.car().unwrap(),
        amlang_node!(f, lang_agent.context()).into()
    );
    assert_eq!(
        *cons.cdr().unwrap(),
        amlang_node!(t, lang_agent.context()).into()
    );
}

#[test]
fn basic_apply() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(apply + '(1 2))");
    assert_eq!(results, vec![Number::I64(3).into()]);
}

#[test]
fn basic_fexpr() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "((fexpr (a) (car (cdr a))) (+ 1 2))");
    assert_eq!(results, vec![Number::I64(1).into()]);
}

#[test]
fn def_atom() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(def a)");
    // Atom should designate to itself.
    assert_eq!(
        lang_agent
            .designate(Primitive::try_from(results[0].clone()).unwrap())
            .unwrap(),
        results[0],
    );
}

#[test]
fn def_number() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(def a 2)");
    assert_eq!(
        lang_agent
            .designate(Primitive::try_from(results[0].clone()).unwrap())
            .unwrap(),
        Number::I64(2).into()
    );
}

#[test]
fn def_lambda() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(def a (lambda (e) (+ e 2))) (a 2)");
    assert_eq!(results[1], Number::I64(4).into());
}

#[test]
fn def_recursive_lambda() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(def fact (lambda (n)
           (if (eq n 1) 1
             (* n (fact (- n 1))))))

         (fact 4)",
    );
    assert_eq!(results[1], Number::I64(24).into());
}

#[test]
fn set_atom() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(def a)
         (set! a 4)
         a

         (set! a)
         a",
    );
    assert_eq!(results[2], Number::I64(4).into());
    let original_atom = Node::try_from(results[0].clone()).unwrap();
    assert_eq!(results[4], original_atom.into());
}

#[test]
fn set_lambda() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(def a)
         (set! a (lambda (a) (+ a a)))
         (a 4)",
    );
    assert_eq!(results[2], Number::I64(8).into());
}

#[test]
fn set_recursive() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(def a 4)
         (set! a (* a 2))
         a",
    );
    assert_eq!(results[2], Number::I64(8).into());
}

#[test]
fn eval() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        "(eval (car '(lambda)))

         (eval '(+ 1 2))",
    );

    assert_eq!(
        results[0],
        amlang_node!(lambda, lang_agent.context()).into()
    );
    assert_eq!(results[1], Number::I64(3).into());
}

#[test]
fn improper_list() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(&mut lang_agent, "(eq '(1 2 . 3) (cons 1 (cons 2 3)))");
    assert_eq!(results[0], amlang_node!(t, lang_agent.context()).into());
}

#[test]
fn basic_ask_tell() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

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
fn import() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results(
        &mut lang_agent,
        ";; Once imported, should return same Node.
         (jump (import lambda))
         (eq (curr) (import lambda))

         ;; Should be idempotent.
         (eq (import lambda) (import (import lambda)))

         ;; Should be false since test agent has own working environment.
         (eq lambda (import lambda))

         ;; Importing from same environment should return original Node.
         (jump lambda)
         (eq lambda (import lambda))",
    );

    let context = lang_agent.context();
    assert_eq!(results[1], amlang_node!(t, context).into());
    assert_eq!(results[2], amlang_node!(t, context).into());
    assert_eq!(results[3], amlang_node!(f, context).into());
    assert_eq!(results[5], amlang_node!(t, context).into());
}

#[test]
fn tell_dupe() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results_with_errors(
        &mut lang_agent,
        "(def a)

         (def related_to)
         (tell a related_to a)
         (tell a related_to a)

         (def is)
         (tell is tell_handler (lambda (s p o) true))
         (tell a is a)
         ;; Dupes rejected prior to reaching handler.
         (tell a is a)",
    );

    let err = results[3].as_ref().unwrap_err().kind().reify();
    let (_, kind, _triple) = break_sexp!(err => (LangString, LangString, Sexp)).unwrap();
    assert_eq!(kind.as_str(), "Duplicate triple");

    let err = results[7].as_ref().unwrap_err().kind().reify();
    let (_, kind, _triple) = break_sexp!(err => (LangString, LangString, Sexp)).unwrap();
    assert_eq!(kind.as_str(), "Duplicate triple");
}

#[test]
fn tell_handler_reject() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results_with_errors(
        &mut lang_agent,
        "(def is)
         (tell is tell_handler (lambda (s p o) false))

         (def a)
         (tell a is a)",
    );

    let err = results[3].as_ref().unwrap_err().kind().reify();
    let (_, kind, _triple, ret) = break_sexp!(err => (LangString, LangString, Sexp, Node)).unwrap();
    assert_eq!(kind.as_str(), "Rejected triple");
    assert_eq!(ret, amlang_node!(f, lang_agent.context()).into());
}

#[test]
fn tell_handler_as_eq() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = common::results_with_errors(
        &mut lang_agent,
        "(def is)
         (def a)
         (def b)

         (tell is tell_handler (lambda (s p o) (eq s o)))

         ;; This should succeed.
         (tell a is a)
         ;; But this shouldn't.
         (tell a is b)",
    );

    assert!(matches!(results[3], Ok(..)));
    assert!(matches!(results[4], Ok(..)));

    let err = results[5].as_ref().unwrap_err().kind().reify();
    let (_, kind, _triple, ret) = break_sexp!(err => (LangString, LangString, Sexp, Node)).unwrap();
    assert_eq!(kind.as_str(), "Rejected triple");
    assert_eq!(ret, amlang_node!(f, lang_agent.context()).into());
}
