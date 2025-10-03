mod bindings {
    use crate::Component;

    wit_bindgen::generate!();

    export!(Component);
}

use bindings::exports::myapp::app::custom::{Guest, TestRecord};

struct Component;

impl Guest for Component {
    fn foo(t: TestRecord) -> u64 {
        assert_eq!(t.foo, "myfoo");
        assert_eq!(t.bar, "mybar");
        42
    }
}
