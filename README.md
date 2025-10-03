```
$ cargo build --target wasm32-unknown-unknown --release --manifest-path ./contract/Cargo.toml
$ cargo run ./contract/target/wasm32-unknown-unknown/release
$ curl -H "X-Contract: contract" -H "X-Func: myapp:app/custom@0.1.0#foo" "localhost:8080" -H "X-Codec: https://github.com/rvolosatovs/wasm-serde/releases/download/poc-1/wasm_serde.wasm" -d '{"foo":"myfoo","bar":"mybar"}'
```
