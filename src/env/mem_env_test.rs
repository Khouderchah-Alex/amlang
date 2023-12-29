use super::*;

use crate::env::entry::EntryMutKind;
use crate::env::mem_backend::SimpleBackend;


#[test]
fn atomic_insertion() {
    let mut env = MemEnv::<SimpleBackend>::new();
    let a = env.insert_node(None);
    let b = env.insert_node(None);
    let c = env.insert_node(None);

    let t = env.insert_triple(a, b, c);
    assert_eq!(env.triple_predicate(t), b);

    let m = env.match_subject(a);
    assert_eq!(m.len(), 1);
    assert_eq!(m.objects().next().unwrap(), c);
}

#[test]
fn structure_insertion() {
    let mut env = MemEnv::<SimpleBackend>::new();
    let a = env.insert_node("(1 2 3)".parse().ok());
    assert_eq!(env.entry(a).structure(), &"(1 2 3)".parse().unwrap());

    let b = env.insert_node(None);
    let c = env.insert_node(None);

    let t = env.insert_triple(a, b, c);
    assert_eq!(env.triple_subject(t), a);
    assert_eq!(
        env.entry(env.triple_subject(t)).structure(),
        &"(1 2 3)".parse().unwrap()
    );

    let m = env.match_predicate(b);
    assert_eq!(m.len(), 1);
    assert_eq!(m.objects().next().unwrap(), c);
}

#[test]
fn entry_update() {
    let mut env = MemEnv::<SimpleBackend>::new();
    let a = env.insert_node(None);

    let mut entry = env.entry_mut(a);
    assert_eq!(*entry.kind(), EntryMutKind::Atomic);

    // Explicitly use update.
    *entry.kind_mut() = EntryMutKind::Owned("(1 2 3)".parse().unwrap());
    entry.update();
    assert_eq!(*env.entry(a).structure(), "(1 2 3)".parse().unwrap());

    // Implicitly use drop.
    *env.entry_mut(a).kind_mut() = EntryMutKind::Atomic;
    assert_eq!(*env.entry(a).kind(), EntryKind::Atomic);
}

#[test]
fn meta_triple_insertion() {
    let mut env = MemEnv::<SimpleBackend>::new();
    let a = env.insert_node(None);
    let b = env.insert_node(None);
    let c = env.insert_node(None);

    let t = env.insert_triple(a, b, c);
    let tt = env.insert_triple(t.node(), a, c);
    assert_eq!(env.triple_subject(tt), t.node());
}
