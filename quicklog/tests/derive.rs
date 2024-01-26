use quicklog::{serialize::Serialize, Serialize};

const fn get_decode<T: quicklog::serialize::Serialize>(_: &T) -> quicklog::serialize::DecodeFn {
    T::decode
}

macro_rules! decode_and_assert {
    ($decode:expr, $buf:expr) => {{
        let (out, rest) = $crate::get_decode(&$decode)($buf).unwrap();
        assert_eq!(format!("{}", $decode), out);
        rest
    }};

    ($decode:expr, $expected:expr, $buf:expr) => {{
        let (out, rest) = $crate::get_decode(&$decode)($buf).unwrap();
        assert_eq!($expected, out);
        rest
    }};
}

#[test]
fn struct_empty() {
    #[derive(Serialize)]
    struct TestStruct;

    let a = TestStruct;
    let mut buf = [0; 128];
    _ = a.encode(&mut buf);

    decode_and_assert!(a, "TestStruct".to_string(), &buf);
}

#[test]
fn struct_primitive() {
    #[derive(Serialize)]
    struct TestStruct {
        size: usize,
    }

    let s = TestStruct { size: 0 };
    let mut buf = [0; 128];

    _ = s.encode(&mut buf);
    decode_and_assert!(s, format!("TestStruct {{ size: {} }}", s.size), &buf);
}

#[test]
fn struct_primitives() {
    #[derive(Serialize)]
    struct TestStruct {
        a: usize,
        b: i32,
        c: u32,
    }

    let s = TestStruct {
        a: 0,
        b: -999,
        c: 2,
    };
    let mut buf = [0; 128];

    _ = s.encode(&mut buf);
    decode_and_assert!(
        s,
        format!("TestStruct {{ a: {}, b: {}, c: {} }}", s.a, s.b, s.c),
        &buf
    );
}

#[test]
fn struct_str() {
    #[derive(Serialize)]
    struct TestStruct<'a> {
        some_str: &'static str,
        another_str: &'a str,
    }

    let another_string = "Hello".to_string() + "there";
    let s = TestStruct {
        some_str: "hello world",
        another_str: another_string.as_str(),
    };
    let mut buf = [0; 128];

    _ = s.encode(&mut buf);
    decode_and_assert!(
        s,
        format!(
            "TestStruct {{ some_str: {}, another_str: {} }}",
            s.some_str, s.another_str
        ),
        &buf
    );
}

#[test]
fn struct_primitives_str() {
    #[derive(Serialize)]
    struct TestStruct {
        a: usize,
        some_str: &'static str,
        b: i32,
    }

    let s = TestStruct {
        a: 999,
        some_str: "hello world",
        b: -32,
    };
    let mut buf = [0; 128];

    _ = s.encode(&mut buf);
    decode_and_assert!(
        s,
        format!(
            "TestStruct {{ a: {}, some_str: {}, b: {} }}",
            s.a, s.some_str, s.b
        ),
        &buf
    );
}

#[test]
fn struct_tuple() {
    #[derive(Serialize)]
    struct TestStruct(usize, &'static str, i32);

    let s = TestStruct(999, "hello world", -32);
    let mut buf = [0; 128];

    _ = s.encode(&mut buf);
    decode_and_assert!(s, format!("TestStruct({}, {}, {})", s.0, s.1, s.2), &buf);
}

#[test]
fn struct_collections() {
    #[derive(Serialize)]
    struct TestStruct {
        a: Vec<String>,
        b: Vec<&'static str>,
    }

    let s = TestStruct {
        a: vec!["1".to_string(), "2".to_string()],
        b: vec!["3", "4", "5"],
    };
    let mut buf = [0; 256];

    _ = s.encode(&mut buf);
    decode_and_assert!(
        s,
        "TestStruct { a: [\"1\", \"2\"], b: [\"3\", \"4\", \"5\"] }".to_string(),
        &buf
    );
}

#[test]
fn enum_no_fields() {
    #[derive(Serialize)]
    enum TestEnum {
        Foo,
        Bar,
        Baz,
    }

    let a = TestEnum::Foo;
    let b = TestEnum::Bar;
    let c = TestEnum::Baz;
    let mut buf = [0; 128];

    let rest = a.encode(&mut buf);
    let rest = b.encode(rest);
    let _ = c.encode(rest);

    let rest = decode_and_assert!(a, "Foo", &buf);
    let rest = decode_and_assert!(b, "Bar", rest);
    _ = decode_and_assert!(c, "Baz", rest);
}

#[test]
fn enum_fields() {
    #[derive(Serialize)]
    enum TestEnum<'a> {
        Foo(String),
        Bar { a: String, b: usize },
        Baz(TestStruct<'a>),
    }

    #[derive(Serialize)]
    struct TestStruct<'a> {
        a: usize,
        b: i32,
        c: &'a [u8],
    }

    let a = TestEnum::Foo("hello world".to_string());
    let b = TestEnum::Bar {
        a: "hello bar".to_string(),
        b: 999,
    };
    let buf = [0_u8; 16];
    let slice = buf.as_slice();
    let c = TestEnum::Baz(TestStruct {
        a: 0,
        b: -999,
        c: slice,
    });
    let mut buf = [0; 256];

    let rest = a.encode(&mut buf);
    let rest = b.encode(rest);
    _ = c.encode(rest);

    let rest = decode_and_assert!(a, "Foo(hello world)", &buf);
    let rest = decode_and_assert!(b, "Bar { a: hello bar, b: 999 }", rest);
    _ = decode_and_assert!(
        c,
        format!("Baz(TestStruct {{ a: 0, b: -999, c: {:?} }})", slice),
        rest
    );
}
