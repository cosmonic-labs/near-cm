use bench::bindings::rvolosatovs::serde::deserializer::Deserializer;
use bench::{assert_deserialize_big_input, assert_deserialize_small_input, bindings};

struct Component;

bindings::export!(Component with_types_in bindings);

impl bindings::Guest for Component {
    fn noop() {}

    fn run_small() {
        assert_deserialize_small_input(Deserializer::from_list(&bindings::input()))
    }

    fn run_big() {
        assert_deserialize_big_input(Deserializer::from_list(&bindings::input()))
    }

    fn run_small_bytes(buf: Vec<u8>) {
        assert_deserialize_small_input(Deserializer::from_list(&buf))
    }

    fn run_big_bytes(buf: Vec<u8>) {
        assert_deserialize_big_input(Deserializer::from_list(&buf))
    }
}
