use bench::bindings::{BigInput, BigInputElement, BigInputElementPayload, SmallInput};
use bench::{assert_big_input, assert_small_input};
use bindings::cosmonic::serde::deserializer;
use bindings::cosmonic::reflect::reflect::{
    List, ListType, RecordType, RecordValue, TupleType, TupleValue, Type, Value,
};
use std::sync::LazyLock;

struct Component;

mod bindings {
    wit_bindgen::generate!({
        path: "../../../wasm-serde/wit",
        world: "imports",
        generate_all,
    });
}

bench::bindings::export!(Component with_types_in bench::bindings);

macro_rules! impl_unwrap_value_primitive {
    ($rt:ident, $f:ident, $f_list:ident, $t:ty) => {
        #[allow(dead_code)]
        #[track_caller]
        pub fn $f(self) -> $t {
            let Value::$rt(v) = self else {
                panic!("invalid value type");
            };
            v
        }

        #[allow(dead_code)]
        #[track_caller]
        pub fn $f_list(self) -> Vec<$t> {
            let Value::List(List::$rt(vs)) = self else {
                panic!("invalid value list type");
            };
            vs
        }
    };
}

impl Value {
    impl_unwrap_value_primitive!(Bool, unwrap_bool, unwrap_list_bool, bool);
    impl_unwrap_value_primitive!(U8, unwrap_u8, unwrap_list_u8, u8);
    impl_unwrap_value_primitive!(U16, unwrap_u16, unwrap_list_u16, u16);
    impl_unwrap_value_primitive!(U32, unwrap_u32, unwrap_list_u32, u32);
    impl_unwrap_value_primitive!(U64, unwrap_u64, unwrap_list_u64, u64);
    impl_unwrap_value_primitive!(S8, unwrap_s8, unwrap_list_s8, i8);
    impl_unwrap_value_primitive!(S16, unwrap_s16, unwrap_list_s16, i16);
    impl_unwrap_value_primitive!(S32, unwrap_s32, unwrap_list_s32, i32);
    impl_unwrap_value_primitive!(S64, unwrap_s64, unwrap_list_s64, i64);
    impl_unwrap_value_primitive!(F32, unwrap_f32, unwrap_list_f32, f32);
    impl_unwrap_value_primitive!(F64, unwrap_f64, unwrap_list_f64, f64);
    impl_unwrap_value_primitive!(Char, unwrap_char, unwrap_list_char, char);
    impl_unwrap_value_primitive!(String, unwrap_string, unwrap_list_string, String);
    impl_unwrap_value_primitive!(Flags, unwrap_flags, unwrap_list_flags, u32);
    impl_unwrap_value_primitive!(Enum, unwrap_enum, unwrap_list_enum, u32);

    #[track_caller]
    pub fn unwrap_record(self) -> Vec<Value> {
        let Value::Record(v) = self else {
            panic!("invalid value type");
        };
        RecordValue::into_value(v)
    }

    #[track_caller]
    pub fn unwrap_tuple(self) -> Vec<Value> {
        let Value::Tuple(v) = self else {
            panic!("invalid value type");
        };
        TupleValue::into_value(v)
    }

    #[track_caller]
    pub fn unwrap_list(self) -> List {
        let Value::List(v) = self else {
            panic!("invalid value type");
        };
        v
    }
}

static SMALL_INPUT_TYPE: LazyLock<RecordType> = LazyLock::new(|| {
    RecordType::new(&[
        ("a".into(), Type::String),
        ("b".into(), Type::U32),
        (
            "c".into(),
            Type::Tuple(&TupleType::new(&[Type::U32, Type::U32, Type::U32])),
        ),
    ])
});

static BIG_INPUT_TYPE: LazyLock<RecordType> = LazyLock::new(|| {
    RecordType::new(&[(
        "signed".into(),
        Type::List(ListType::Record(&RecordType::new(&[
            (
                "payload".into(),
                Type::Record(&RecordType::new(&[
                    ("nonce".into(), Type::String),
                    ("message".into(), Type::String),
                    ("recipient".into(), Type::String),
                ])),
            ),
            ("standard".into(), Type::String),
            ("signature".into(), Type::String),
            ("public_key".into(), Type::String),
        ]))),
    )])
});

pub fn assert_unwrap_small_input(v: Value) {
    let fields = v.unwrap_record();
    let mut fields = fields.into_iter();

    let a = fields.next().unwrap().unwrap_string();
    let b = fields.next().unwrap().unwrap_u32();
    let c_values = fields.next().unwrap().unwrap_tuple();
    let mut c_values = c_values.into_iter();

    let c0 = c_values.next().unwrap().unwrap_u32();
    let c1 = c_values.next().unwrap().unwrap_u32();
    let c2 = c_values.next().unwrap().unwrap_u32();

    assert_small_input(SmallInput {
        a,
        b,
        c: (c0, c1, c2),
    })
}

pub fn unwrap_big_input_element_payload(
    fields: impl IntoIterator<Item = Value>,
) -> BigInputElementPayload {
    let mut fields = fields.into_iter();

    let nonce = fields.next().unwrap().unwrap_string();
    let message = fields.next().unwrap().unwrap_string();
    let recipient = fields.next().unwrap().unwrap_string();

    BigInputElementPayload {
        nonce,
        message,
        recipient,
    }
}

pub fn unwrap_big_input_element(fields: impl IntoIterator<Item = Value>) -> BigInputElement {
    let mut fields = fields.into_iter();

    let payload = fields.next().unwrap().unwrap_record();
    let payload = unwrap_big_input_element_payload(payload);

    let standard = fields.next().unwrap().unwrap_string();
    let signature = fields.next().unwrap().unwrap_string();
    let public_key = fields.next().unwrap().unwrap_string();

    BigInputElement {
        payload,
        standard,
        signature,
        public_key,
    }
}

pub fn unwrap_big_input(v: Value) -> BigInput {
    let fields = v.unwrap_record();
    let mut fields = fields.into_iter();
    let List::Record(elems) = fields.next().unwrap().unwrap_list() else {
        panic!()
    };

    let mut signed = Vec::with_capacity(elems.len());
    for el in elems {
        let el = unwrap_big_input_element(RecordValue::into_value(el));
        signed.push(el);
    }
    BigInput { signed }
}

pub fn assert_unwrap_big_input(v: Value) {
    let v = unwrap_big_input(v);
    assert_big_input(v);
}

impl bench::bindings::Guest for Component {
    fn noop() {}

    fn run_small() {
        let buf = bench::bindings::input();
        let v = deserializer::from_list(&buf, Type::Record(&SMALL_INPUT_TYPE)).unwrap();
        assert_unwrap_small_input(v)
    }

    fn run_big() {
        let buf = bench::bindings::input();
        let v = deserializer::from_list(&buf, Type::Record(&BIG_INPUT_TYPE)).unwrap();
        assert_unwrap_big_input(v)
    }

    fn run_small_bytes(buf: Vec<u8>) {
        let v = deserializer::from_list(&buf, Type::Record(&SMALL_INPUT_TYPE)).unwrap();
        assert_unwrap_small_input(v)
    }

    fn run_big_bytes(buf: Vec<u8>) {
        let v = deserializer::from_list(&buf, Type::Record(&BIG_INPUT_TYPE)).unwrap();
        assert_unwrap_big_input(v)
    }

    fn run_small_typed(v: SmallInput) {
        assert_small_input(v)
    }

    fn run_big_typed(v: BigInput) {
        assert_big_input(v)
    }
}
