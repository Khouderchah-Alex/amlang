use super::*;


#[test]
fn basic_copy() {
    let v = AppendVec::<usize>::new(2);
    unsafe {
        assert!(v.push(10).is_none());
        assert!(v.push(20).is_none());
        let new = v.push(30).unwrap();

        assert_eq!(v.len(), 2);
        assert_eq!(new.len(), 3);

        assert_eq!(v[1], new[1]);

        let mut iter = new.iter();
        assert_eq!(iter.next().cloned(), Some(10));
        assert_eq!(iter.next().cloned(), Some(20));
        assert_eq!(iter.next().cloned(), Some(30));
        assert_eq!(iter.next().cloned(), None);
    }
}

#[test]
fn basic_no_copy() {
    let v = AppendVec::<String>::new(2);
    unsafe {
        assert!(v.push("hello".to_string()).is_none());
        assert!(v.push("bye".to_string()).is_none());
        let new = v.push("t".to_string()).unwrap();

        assert_eq!(v.len(), 2);
        assert_eq!(new.len(), 3);

        assert_eq!(v[1], new[1]);

        let mut iter = new.iter();
        assert_eq!(iter.next().cloned(), Some("hello".to_string()));
        assert_eq!(iter.next().cloned(), Some("bye".to_string()));
        assert_eq!(iter.next().cloned(), Some("t".to_string()));
        assert_eq!(iter.next().cloned(), None);
    }
}
