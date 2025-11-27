pub mod bindings {
    wit_bindgen::generate!({
        path: "../wit",
        pub_export_macro: true,
        ownership: Borrowing { duplicate_if_necessary: true },
        generate_all,
        additional_derives: [serde::Deserialize, Eq, PartialEq],
    });
}

use bindings::{BigInput, SmallInput};

pub fn assert_small_input(SmallInput { a, b, c }: SmallInput) {
    assert_eq!(a, "test");
    assert_eq!(b, 42);
    assert_eq!(c, (0, 1, 2));
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
