mod bindings {
    use crate::Component;

    wit_bindgen::generate!();

    export!(Component);
}

use bindings::exports::myapp::app::custom::{Guest, TestRecord};

struct Component;

impl Guest for Component {
    fn greet(s: String) -> String {
        format!("Hello, {s}!")
    }

    fn add(a: u64, b: u64) -> u64 {
        a.saturating_add(b)
    }

    fn foo(t: TestRecord) -> u64 {
        assert_eq!(t.foo, "myfoo");
        assert_eq!(t.bar, "mybar");
        bindings::mul(6, 7)
    }
}
