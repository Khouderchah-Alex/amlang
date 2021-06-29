use super::*;


#[test]
fn atomic_insertion() {
    let mut env = MemEnvironment::new();
    let a = env.insert_atom();
    let b = env.insert_atom();
    let c = env.insert_atom();

    let t = env.insert_triple(a, b, c);
    assert_eq!(env.triple_predicate(t), b);

    let m = env.match_subject(a);
    assert_eq!(m.len(), 1);
    assert_eq!(env.triple_object(*m.iter().next().unwrap()), c);
}

#[test]
fn structure_insertion() {
    let mut env = MemEnvironment::new();
    let a = env.insert_structure("(1 2 3)".parse().unwrap());
    assert_eq!(env.node_structure(a).unwrap(), &"(1 2 3)".parse().unwrap());

    let b = env.insert_atom();
    let c = env.insert_atom();

    let t = env.insert_triple(a, b, c);
    assert_eq!(env.triple_subject(t), a);
    assert_eq!(
        env.node_structure(env.triple_subject(t)).unwrap(),
        &"(1 2 3)".parse().unwrap()
    );

    let m = env.match_predicate(b);
    assert_eq!(m.len(), 1);
    assert_eq!(env.triple_object(*m.iter().next().unwrap()), c);
}
