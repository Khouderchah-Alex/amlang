use super::*;


#[test]
fn struct_() {
    #[allow(dead_code)]
    #[derive(serde::Deserialize, Debug)]
    struct TestStruct {
        #[serde(rename = "g")]
        a: Option<i64>,
        b: i128,
        c: u128,
        d: D,
    }

    #[derive(serde::Deserialize, Debug)]
    enum D {
        #[serde(rename = "f")]
        A,
        B,
    }

    let i = Introspection::of::<TestStruct>();
    assert_eq!(i.name(), "TestStruct");
    let names = i.fields();
    assert_eq!(names[0], "g");
    assert_eq!(names[1], "b");
    assert_eq!(names[2], "c");
    assert_eq!(names[3], "d");
}

#[test]
fn enum_() {
    #[derive(serde::Deserialize, Debug)]
    #[serde(rename = "MyEnum")]
    enum TestEnum {
        #[serde(rename = "f")]
        A,
        B,
    }

    let i = Introspection::of::<TestEnum>();
    assert_eq!(i.name(), "MyEnum");
    let names = i.fields();
    assert_eq!(names[0], "f");
    assert_eq!(names[1], "B");
}

#[test]
fn newtype() {
    #[derive(serde::Deserialize, Debug)]
    struct NewInt(i32);

    let i = Introspection::of::<NewInt>();
    assert_eq!(i.name(), "NewInt");
    let names = i.fields();
    assert_eq!(names.len(), 0);
}
