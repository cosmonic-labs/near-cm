use bench::bindings::{BigInput, BigInputElement, BigInputElementPayload, SmallInput};
use bench::{assert_big_input, assert_small_input};
use bindings::rvolosatovs::serde::deserializer::{
    Deserializer, ListDeserializer, RecordDeserializer, TupleDeserializer,
};
use bindings::rvolosatovs::serde::reflect::{ListType, RecordType, TupleType, Type};
use std::sync::LazyLock;

struct Component;

mod bindings {
    wit_bindgen::generate!({
        path: "../../../wasm-serde/wit",
        world: "imports",
    });
}

bench::bindings::export!(Component with_types_in bench::bindings);

static SMALL_INPUT_C_TYPE: LazyLock<TupleType> =
    LazyLock::new(|| TupleType::new(&[Type::U32, Type::U32, Type::U32]));

static SMALL_INPUT_TYPE: LazyLock<RecordType> = LazyLock::new(|| {
    RecordType::new(&[
        ("a".into(), Type::String),
        ("b".into(), Type::U32),
        ("c".into(), Type::Tuple(&SMALL_INPUT_C_TYPE)),
    ])
});

static BIG_INPUT_ELEMENT_PAYLOAD_TYPE: LazyLock<RecordType> = LazyLock::new(|| {
    RecordType::new(&[
        ("nonce".into(), Type::String),
        ("message".into(), Type::String),
        ("recipient".into(), Type::String),
    ])
});

static BIG_INPUT_ELEMENT_TYPE: LazyLock<RecordType> = LazyLock::new(|| {
    RecordType::new(&[
        (
            "payload".into(),
            Type::Record(&BIG_INPUT_ELEMENT_PAYLOAD_TYPE),
        ),
        ("standard".into(), Type::String),
        ("signature".into(), Type::String),
        ("public_key".into(), Type::String),
    ])
});

static BIG_INPUT_SIGNED_TYPE: LazyLock<ListType> =
    LazyLock::new(|| ListType::new(&Type::Record(&BIG_INPUT_ELEMENT_TYPE)));

static BIG_INPUT_TYPE: LazyLock<RecordType> =
    LazyLock::new(|| RecordType::new(&[("signed".into(), Type::List(&BIG_INPUT_SIGNED_TYPE))]));

pub fn assert_deserialize_small_input(de: Deserializer) {
    let (idx, a, de) = Deserializer::deserialize_record(de, &SMALL_INPUT_TYPE).unwrap();
    assert_eq!(idx, 0);
    let a = Deserializer::deserialize_string(a).unwrap();

    let (idx, b, de) = RecordDeserializer::next(de);
    assert_eq!(idx, 1);
    let b = Deserializer::deserialize_u32(b).unwrap();

    let (idx, c, _) = RecordDeserializer::next(de);
    assert_eq!(idx, 2);

    let (c0, c_de) = Deserializer::deserialize_tuple(c, &SMALL_INPUT_C_TYPE).unwrap();
    let c0 = Deserializer::deserialize_u32(c0).unwrap();

    let (c1, c_de) = TupleDeserializer::next(c_de);
    let c1 = Deserializer::deserialize_u32(c1).unwrap();

    let (c2, _) = TupleDeserializer::next(c_de);
    let c2 = Deserializer::deserialize_u32(c2).unwrap();

    assert_small_input(SmallInput {
        a,
        b,
        c: (c0, c1, c2),
    })
}

pub fn deserialize_big_input_element_payload(de: Deserializer) -> BigInputElementPayload {
    let (idx, nonce, de) =
        Deserializer::deserialize_record(de, &BIG_INPUT_ELEMENT_PAYLOAD_TYPE).unwrap();
    assert_eq!(idx, 0);
    let nonce = Deserializer::deserialize_string(nonce).unwrap();

    let (idx, message, de) = RecordDeserializer::next(de);
    assert_eq!(idx, 1);
    let message = Deserializer::deserialize_string(message).unwrap();

    let (idx, recipient, _de) = RecordDeserializer::next(de);
    assert_eq!(idx, 2);
    let recipient = Deserializer::deserialize_string(recipient).unwrap();

    BigInputElementPayload {
        nonce,
        message,
        recipient,
    }
}

pub fn deserialize_big_input_element(de: Deserializer) -> BigInputElement {
    let (idx, payload, de) = Deserializer::deserialize_record(de, &BIG_INPUT_ELEMENT_TYPE).unwrap();
    assert_eq!(idx, 0);
    let payload = deserialize_big_input_element_payload(payload);

    let (idx, standard, de) = RecordDeserializer::next(de);
    assert_eq!(idx, 1);
    let standard = Deserializer::deserialize_string(standard).unwrap();

    let (idx, signature, de) = RecordDeserializer::next(de);
    assert_eq!(idx, 2);
    let signature = Deserializer::deserialize_string(signature).unwrap();

    let (idx, public_key, _) = RecordDeserializer::next(de);
    assert_eq!(idx, 3);
    let public_key = Deserializer::deserialize_string(public_key).unwrap();

    BigInputElement {
        payload,
        standard,
        signature,
        public_key,
    }
}

pub fn deserialize_big_input(de: Deserializer) -> BigInput {
    let (idx, signed, _) = Deserializer::deserialize_record(de, &BIG_INPUT_TYPE).unwrap();
    assert_eq!(idx, 0);

    let mut elems =
        Deserializer::deserialize_list(signed, &Type::Record(&BIG_INPUT_ELEMENT_TYPE)).unwrap();
    let mut signed = Vec::default();
    while let Some((de, next)) = ListDeserializer::next(elems) {
        let el = deserialize_big_input_element(de);
        signed.push(el);
        elems = next;
    }
    BigInput { signed }
}

pub fn assert_deserialize_big_input(de: Deserializer) {
    let v = deserialize_big_input(de);
    assert_big_input(v);
}

impl bench::bindings::Guest for Component {
    fn noop() {}

    fn run_small() {
        assert_deserialize_small_input(Deserializer::from_list(&bench::bindings::input()))
    }

    fn run_big() {
        assert_deserialize_big_input(Deserializer::from_list(&bench::bindings::input()))
    }

    fn run_small_bytes(buf: Vec<u8>) {
        assert_deserialize_small_input(Deserializer::from_list(&buf))
    }

    fn run_big_bytes(buf: Vec<u8>) {
        assert_deserialize_big_input(Deserializer::from_list(&buf))
    }

    fn run_small_typed(v: SmallInput) {
        assert_small_input(v)
    }

    fn run_big_typed(v: BigInput) {
        assert_big_input(v)
    }
}
