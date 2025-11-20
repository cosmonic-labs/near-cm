use bindings::rvolosatovs::serde::deserializer::{
    Deserializer, ListDeserializer, RecordDeserializer, TupleDeserializer,
};
use serde::Deserialize;

pub mod bindings {
    wit_bindgen::generate!({
        path: "../wit",
        pub_export_macro: true,
        ownership: Borrowing { duplicate_if_necessary: true },
        generate_all,
    });
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct SmallInput {
    pub a: String,
    pub b: u32,
    pub c: [u32; 3],
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct BigInputElementPayload {
    pub nonce: String,
    pub message: String,
    pub recipient: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct BigInputElement {
    pub payload: BigInputElementPayload,
    pub standard: String,
    pub signature: String,
    pub public_key: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct BigInput {
    pub signed: Vec<BigInputElement>,
}

pub fn assert_small_input(SmallInput { a, b, c }: SmallInput) {
    assert_eq!(a, "test");
    assert_eq!(b, 42);
    assert_eq!(c, [0, 1, 2]);
}

pub fn assert_big_input(BigInput { signed }: BigInput) {
    let [ref first, ref second] = *signed else {
        panic!()
    };
    assert_eq!(
        first.payload.nonce,
        "XCkavXk45BCln15mDa50zMN+uWXqv6nVTFbY4vi3b9Y="
    );
    assert_eq!(
        first.payload.message,
        r#"{"signer_id":"1d3c4c1898200faa3273e06b1834098ec635c88e538aeceb095d18321861a970","deadline":"2025-09-23T14:42:10.476Z","intents":[{"intent":"token_diff","diff":{"nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1":"999999000","nep141:wrap.near":"-327580166752348350287445024"}}]}"#
    );
    assert_eq!(first.payload.recipient, "intents.near");
    assert_eq!(first.standard, "nep413");
    assert_eq!(
        first.signature,
        "ed25519:Scq7yw8YPWEwni9Rvy8R9pEFUCmscUSkRAu2LB9grPcr6L1NoNELBtiZZ58wm1cDrsgWForeaADkHnVmaqE6ULP"
    );
    assert_eq!(
        first.public_key,
        "ed25519:6Eb6wkNagkMg5EfZjo2AmStrUrxPKWLSHYDqVX7ofxtV"
    );

    assert_eq!(
        second.payload.nonce,
        "7vX4hxEe9Hu5veUyivPWPxDHpYNsXzi7EG8bc05EIlA="
    );
    assert_eq!(
        second.payload.message,
        r#"{"signer_id":"foxboss.near","deadline":"2025-09-23T14:42:10.476Z","intents":[{"intent":"token_diff","diff":{"nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1":"-1000000000","nep141:wrap.near":"327579839172181597939094736"}}]}"#
    );
    assert_eq!(second.payload.recipient, "intents.near");
    assert_eq!(second.standard, "nep413");
    assert_eq!(
        second.signature,
        "ed25519:5Md212LF1YcemtoGCUPfB9sv1pijraj2VkKrFJscawXA7XXuVg6hvWTPcAvz2CuBH8Ns16Rmik1n7r9ySyyQJqWY"
    );
    assert_eq!(
        second.public_key,
        "ed25519:ESzYnUQTyVsNpk2u8ZHZ6q2MF8MHsuoKFRC4aqYAvcJD"
    );
}

pub fn assert_deserialize_small_input(de: Deserializer) {
    let (idx, a, de) = Deserializer::deserialize_record(de, &["a", "b", "c"]).unwrap();
    assert_eq!(idx, 0);
    let a = Deserializer::deserialize_string(a).unwrap();

    let (idx, b, de) = RecordDeserializer::next(de);
    assert_eq!(idx, 1);
    let b = Deserializer::deserialize_u32(b).unwrap();

    let (idx, c, _) = RecordDeserializer::next(de);
    assert_eq!(idx, 2);

    let (c0, c_de) = Deserializer::deserialize_tuple(c, 3).unwrap();
    let c0 = Deserializer::deserialize_u32(c0).unwrap();

    let (c1, c_de) = TupleDeserializer::next(c_de);
    let c1 = Deserializer::deserialize_u32(c1).unwrap();

    let (c2, _) = TupleDeserializer::next(c_de);
    let c2 = Deserializer::deserialize_u32(c2).unwrap();

    assert_small_input(SmallInput {
        a,
        b,
        c: [c0, c1, c2],
    })
}

pub fn deserialize_big_input_element_payload(de: Deserializer) -> BigInputElementPayload {
    let (idx, nonce, de) =
        Deserializer::deserialize_record(de, &["nonce", "message", "recipient"]).unwrap();
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
    let (idx, payload, de) =
        Deserializer::deserialize_record(de, &["payload", "standard", "signature", "public_key"])
            .unwrap();
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
    let (idx, signed, _) = Deserializer::deserialize_record(de, &["signed"]).unwrap();
    assert_eq!(idx, 0);

    let mut elems = Deserializer::deserialize_list(signed).unwrap();
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
