mod bindings {
    use crate::Component;

    wit_bindgen::generate!();

    export!(Component);
}

struct Component;

impl bindings::Guest for Component {
    fn mul(x: u64, y: u64) -> u64 {
        x * y
    }
}
