use super::*;


#[test]
fn contains_self() {
    let env = MemEnvironment::new();
    assert_eq!(env.node_structure(env.self_node()), None);
    assert_eq!(env.node_as_triple(env.self_node()), None);
}

#[test]
fn atomic_insertion() {
    let mut env = MemEnvironment::new();
    let a = env.insert_atom();
    let b = env.insert_atom();

    let t = env.insert_triple(env.self_node(), a, b);
    assert_eq!(env.triple_predicate(t), a);

    let m = env.match_subject(env.self_node());
    assert_eq!(m.len(), 1);
    assert_eq!(env.triple_object(*m.iter().next().unwrap()), b);
}

#[test]
fn structure_insertion() {
    let mut env = MemEnvironment::new();
    let a = env.insert_structure("(1 2 3)".parse::<Sexp>().unwrap());
    assert_eq!(
        env.node_structure(a).unwrap(),
        &"(1 2 3)".parse::<Sexp>().unwrap()
    );

    let b = env.insert_atom();

    let t = env.insert_triple(env.self_node(), a, b);
    assert_eq!(env.triple_predicate(t), a);
    assert_eq!(
        env.node_structure(env.triple_predicate(t)).unwrap(),
        &"(1 2 3)".parse::<Sexp>().unwrap()
    );

    let m = env.match_subject(env.self_node());
    assert_eq!(m.len(), 1);
    assert_eq!(env.triple_object(*m.iter().next().unwrap()), b);
}
