use bench::bindings::{BigInput, SmallInput};
use bench::{assert_big_input, assert_small_input, bindings};

struct Component;

bindings::export!(Component with_types_in bindings);

impl bindings::Guest for Component {
    fn noop() {}

    fn run_small() {
        let v = serde_json::from_slice(&bindings::input()).unwrap();
        assert_small_input(v)
    }

    fn run_big() {
        let v = serde_json::from_slice(&bindings::input()).unwrap();
        assert_big_input(v)
    }

    fn run_small_bytes(buf: Vec<u8>) {
        let v = serde_json::from_slice(&buf).unwrap();
        assert_small_input(v)
    }

    fn run_big_bytes(buf: Vec<u8>) {
        let v = serde_json::from_slice(&buf).unwrap();
        assert_big_input(v)
    }

    fn run_small_typed(v: SmallInput) {
        assert_small_input(v)
    }

    fn run_big_typed(v: BigInput) {
        assert_big_input(v)
    }
}
