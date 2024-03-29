mod common;

use std::convert::TryFrom;

use amlang::agent::TransformExecutor;
use amlang::env::LocalNode;
use amlang::parser::Parser;
use amlang::prelude::*;
use amlang::stream::input::StringReader;
use amlang::token::Tokenizer;


pub fn eval<S: AsRef<str>>(lang_agent: &mut Agent, s: S) -> Vec<Sexp> {
    pull_transform!(?unwrap
                    StringReader::new(s.as_ref())
                    =>> Tokenizer::new(policy_base)
                    =>. Parser::new()
                    =>. TransformExecutor::interpret(lang_agent))
    .map(|e| e.unwrap())
    .collect::<Vec<_>>()
}

pub fn eval_with_errors<S: AsRef<str>>(lang_agent: &mut Agent, s: S) -> Vec<Result<Sexp, Error>> {
    pull_transform!(?unwrap
                    StringReader::new(s.as_ref())
                    =>> Tokenizer::new(policy_base)
                    =>. Parser::new()
                    =>. TransformExecutor::interpret(lang_agent))
    .collect::<Vec<_>>()
}


#[test]
fn basic_arithmetic() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "(+ 1 2) (+ 2 2)");
    assert_eq!(results, vec![3.into(), 4.into()]);

    let results = eval(
        &mut lang_agent,
        "(* (+ 1 1) 3)
         (* (+ 1. 1.) 3.)",
    );
    assert_eq!(results, vec![6.into(), 6.0.into()]);

    let results = eval(
        &mut lang_agent,
        "(/ (- 1 1) 2)
         (/ (+ 1 1) 2)",
    );
    assert_eq!(results, vec![0.into(), 1.into()]);
}

#[test]
fn lambda_param_node_body() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    // Requires concretization to work properly to avoid returning the abstract
    // param node itself.
    let results = eval(&mut lang_agent, "((lambda (a) a) 4)");
    assert_eq!(results, vec![4.into()]);
}

#[test]
fn lambda_single_body() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "((lambda (a) (+ a a)) 4)");
    assert_eq!(results, vec![8.into()]);
}

#[test]
fn lambda_seq_body() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "((lambda (a) (jump a) (curr)) lambda)");
    assert_eq!(
        results,
        vec![
            lang_agent
                .resolve_name(&"lambda".to_symbol_or_panic(policy_base))
                .unwrap()
                .into()
        ]
    );
}

#[test]
fn lambda_branch_body() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(
        &mut lang_agent,
        "(def a (lambda (e) (if (eq e 1) 0 (+ e 2))))
         (a 1)
         (a 2)",
    );
    assert_eq!(results[1], 0.into());
    assert_eq!(results[2], 4.into());
}

#[test]
fn lambda_nested_exec() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "((lambda (a) (+ a a)) (* 4 2))");
    assert_eq!(results, vec![16.into()]);
}

#[test]
fn lambda_proc() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "((lambda (a b) (a b 4)) + 40)");
    assert_eq!(results, vec![44.into()]);
}

#[test]
fn lambda_duplicate_argname() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval_with_errors(&mut lang_agent, "(lambda (a a) (+ a a))");

    let err = results[0].as_ref().unwrap_err().kind().reify();
    let (_, kind, _) = break_sexp!(err => (LangString, LangString; remainder)).unwrap();
    assert_eq!(kind.as_str(), "InvalidArgument");
}

#[test]
fn let_basic() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(
        &mut lang_agent,
        "(let ((a 2)
               (b 4))
           (+ a b))",
    );
    assert_eq!(results, vec![6.into()]);
}

#[test]
fn let_rec_vals() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(
        &mut lang_agent,
        "(letrec ((a 2)
                  (b a)
                  (c b))
           (+ a b c))",
    );
    assert_eq!(results, vec![6.into()]);
}

#[test]
fn let_rec_lambdas() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(
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
        lang_agent
            .resolve_name(&"false".to_symbol_or_panic(policy_base))
            .unwrap()
            .into()
    );
    assert_eq!(
        *cons.cdr().unwrap(),
        lang_agent
            .resolve_name(&"true".to_symbol_or_panic(policy_base))
            .unwrap()
            .into()
    );
}

#[test]
fn basic_apply() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "(apply + '(1 2))");
    assert_eq!(results, vec![3.into()]);
}

#[test]
fn basic_fexpr() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "((fexpr (a) (car (cdr a))) (+ 1 2))");
    assert_eq!(results, vec![1.into()]);
}

#[test]
fn def_atom() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "(def a)");
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

    let results = eval(&mut lang_agent, "(def a 2)");
    assert_eq!(
        lang_agent
            .designate(Primitive::try_from(results[0].clone()).unwrap())
            .unwrap(),
        2.into()
    );
}

#[test]
fn def_lambda() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "(def a (lambda (e) (+ e 2))) (a 2)");
    assert_eq!(results[1], 4.into());
}

#[test]
fn def_recursive_lambda() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(
        &mut lang_agent,
        "(def fact (lambda (n)
           (if (eq n 1) 1
             (* n (fact (- n 1))))))

         (fact 4)",
    );
    assert_eq!(results[1], 24.into());
}


#[test]
fn node_atom() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "(anon)");
    // Atom should designate to itself.
    assert_eq!(
        lang_agent
            .designate(Primitive::try_from(results[0].clone()).unwrap())
            .unwrap(),
        results[0],
    );
}

#[test]
fn node_apply() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "(anon (+ 1 2))");
    assert_eq!(
        lang_agent
            .designate(Primitive::try_from(results[0].clone()).unwrap())
            .unwrap(),
        3.into()
    );
}

#[test]
fn node_recursive() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "(anon (cons 'a $))");
    let infinite = lang_agent
        .designate(Primitive::try_from(results[0].clone()).unwrap())
        .unwrap();

    // Node should be infinite list of 'a, with circularity traversed through `eval`.
    let (car, cdr) = break_sexp!(infinite.clone() => (Symbol; remainder)).unwrap();
    assert_eq!(car, "a".to_symbol_or_panic(policy_base));
    assert_eq!(lang_agent.interpret(*cdr.unwrap()).unwrap(), infinite);
}

#[test]
fn set_atom() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(
        &mut lang_agent,
        "(def a)
         (set! a 4)
         a

         (set! a)
         a",
    );
    assert_eq!(results[2], 4.into());
    let original_atom = Node::try_from(results[0].clone()).unwrap();
    assert_eq!(results[4], original_atom.into());
}

#[test]
fn set_lambda() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(
        &mut lang_agent,
        "(def a)
         (set! a (lambda (a) (+ a a)))
         (a 4)",
    );
    assert_eq!(results[2], 8.into());
}

#[test]
fn set_recursive() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(
        &mut lang_agent,
        "(def a 4)
         (set! a (* a 2))
         a",
    );
    assert_eq!(results[2], 8.into());
}

#[test]
fn eval_() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(
        &mut lang_agent,
        "(eval (car '(lambda)))

         (eval '(+ 1 2))",
    );

    assert_eq!(
        results[0],
        lang_agent
            .resolve_name(&"lambda".to_symbol_or_panic(policy_base))
            .unwrap()
            .into()
    );
    assert_eq!(results[1], 3.into());
}

#[test]
fn improper_list() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, "(eq '(1 2 . 3) (cons 1 (cons 2 3)))");
    assert_eq!(
        results[0],
        lang_agent
            .resolve_name(&"true".to_symbol_or_panic(policy_base))
            .unwrap()
            .into()
    );
}

#[test]
fn basic_ask_tell() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(
        &mut lang_agent,
        "(jump lambda)
         (ask lambda _ _)
         (def VariantOf)
         (def Procedure)
         (tell lambda VariantOf Procedure)
         (ask lambda _ _)
         (ask _ VariantOf _)",
    );

    assert_eq!(results[1].iter().count(), 0);
    assert_eq!(results[5].iter().count(), 1);
    assert_eq!(results[6].iter().count(), 1);
}

#[test]
fn import() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(
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

    let t = lang_agent
        .resolve_name(&"true".to_symbol_or_panic(policy_base))
        .unwrap()
        .into();
    let f = lang_agent
        .resolve_name(&"false".to_symbol_or_panic(policy_base))
        .unwrap()
        .into();
    assert_eq!(results[1], t);
    assert_eq!(results[2], t);
    assert_eq!(results[3], f);
    assert_eq!(results[5], t);
}

#[test]
fn tell_dupe() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval_with_errors(
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
    assert_eq!(kind.as_str(), "DuplicateTriple");

    let err = results[7].as_ref().unwrap_err().kind().reify();
    let (_, kind, _triple) = break_sexp!(err => (LangString, LangString, Sexp)).unwrap();
    assert_eq!(kind.as_str(), "DuplicateTriple");
}

/*
#[test]
fn tell_handler_reject() {
let (mut lang_agent, _manager) = common::setup().unwrap();

let results = eval_with_errors(
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

    let results = eval_with_errors(
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
*/

#[test]
fn env_find() {
    let (mut lang_agent, _manager) = common::setup().unwrap();

    let results = eval(&mut lang_agent, r##"(env-find "lang.env")"##);

    let lang_env = lang_agent.find_env("lang.env").unwrap();
    assert_eq!(results[0], Node::new(LocalNode::default(), lang_env).into());
}
