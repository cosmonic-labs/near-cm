### Build and Run

```
$ cargo build --target wasm32-unknown-unknown --release --manifest-path ./contract/Cargo.toml
$ cargo run ./contract/target/wasm32-unknown-unknown/release
```

### Query

```
$ curl localhost:8080 -H "X-Contract: contract"
```

> myapp:app/custom@0.1.0#greet: func(s: string) -> string
>
> myapp:app/custom@0.1.0#add: func(a: u64, b: u64) -> u64
>
> myapp:app/custom@0.1.0#foo: func(t: record{foo: string, bar: string}) -> u64

### Invocation

```
$ curl localhost:8080 -H "X-Contract: contract" -H "X-Func: myapp:app/custom@0.1.0#greet" -H "X-Codec: https://github.com/rvolosatovs/wasm-serde/releases/download/poc-2/wasm_serde.wasm" -d '["world"]'
```

> [String("Hello, world!")]


```
$ curl localhost:8080 -H "X-Contract: contract" -H "X-Func: myapp:app/custom@0.1.0#add" -H "X-Codec: https://github.com/rvolosatovs/wasm-serde/releases/download/poc-2/wasm_serde.wasm" -d '[3, 5]'
```

> [U64(8)]


```
$ curl localhost:8080 -H "X-Contract: contract" -H "X-Func: myapp:app/custom@0.1.0#foo" -H "X-Codec: https://github.com/rvolosatovs/wasm-serde/releases/download/poc-2/wasm_serde.wasm" -d '[{"foo":"myfoo","bar":"mybar"}]'
```

> [U64(42)]
