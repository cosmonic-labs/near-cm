use core::ops::{Deref, DerefMut};

use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

use anyhow::Context as _;
use criterion::measurement::Measurement;
use criterion::{BatchSize, BenchmarkGroup, Criterion};
use wac_graph::types::Package;
use wac_graph::{CompositionGraph, EncodeOptions};
use wasmtime::component::{Component, HasSelf};
use wasmtime::{Caller, Engine, Extern, Module, ModuleExport, Store, component};
use wit_component::ComponentEncoder;

use crate::codec_bindings::exports::rvolosatovs::serde::reflect;

const SMALL_INPUT: &[u8] = br#"{"a": "test", "b": 42, "c": [0, 1, 2] }"#;

// https://pikespeak.ai/transaction-viewer/8rEAAvvj1SNB7fn7aczUo79k4niyNVtDWkm6FMyDWAUb
const BIG_INPUT: &[u8] = br#"{
  "signed": [
    {
      "payload": {
        "nonce": "XCkavXk45BCln15mDa50zMN+uWXqv6nVTFbY4vi3b9Y=",
        "message": "{\"signer_id\":\"1d3c4c1898200faa3273e06b1834098ec635c88e538aeceb095d18321861a970\",\"deadline\":\"2025-09-23T14:42:10.476Z\",\"intents\":[{\"intent\":\"token_diff\",\"diff\":{\"nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1\":\"999999000\",\"nep141:wrap.near\":\"-327580166752348350287445024\"}}]}",
        "recipient": "intents.near"
      },
      "standard": "nep413",
      "signature": "ed25519:Scq7yw8YPWEwni9Rvy8R9pEFUCmscUSkRAu2LB9grPcr6L1NoNELBtiZZ58wm1cDrsgWForeaADkHnVmaqE6ULP",
      "public_key": "ed25519:6Eb6wkNagkMg5EfZjo2AmStrUrxPKWLSHYDqVX7ofxtV"
    },
    {
      "payload": {
        "nonce": "7vX4hxEe9Hu5veUyivPWPxDHpYNsXzi7EG8bc05EIlA=",
        "message": "{\"signer_id\":\"foxboss.near\",\"deadline\":\"2025-09-23T14:42:10.476Z\",\"intents\":[{\"intent\":\"token_diff\",\"diff\":{\"nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1\":\"-1000000000\",\"nep141:wrap.near\":\"327579839172181597939094736\"}}]}",
        "recipient": "intents.near"
      },
      "standard": "nep413",
      "signature": "ed25519:5Md212LF1YcemtoGCUPfB9sv1pijraj2VkKrFJscawXA7XXuVg6hvWTPcAvz2CuBH8Ns16Rmik1n7r9ySyyQJqWY",
      "public_key": "ed25519:ESzYnUQTyVsNpk2u8ZHZ6q2MF8MHsuoKFRC4aqYAvcJD"
    }
  ]
}"#;

const BIG_INPUT_FIRST_NONCE: &str = "XCkavXk45BCln15mDa50zMN+uWXqv6nVTFbY4vi3b9Y=";
const BIG_INPUT_FIRST_MESSAGE: &str = r#"{"signer_id":"1d3c4c1898200faa3273e06b1834098ec635c88e538aeceb095d18321861a970","deadline":"2025-09-23T14:42:10.476Z","intents":[{"intent":"token_diff","diff":{"nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1":"999999000","nep141:wrap.near":"-327580166752348350287445024"}}]}"#;
const BIG_INPUT_FIRST_SIGNATURE: &str = "ed25519:Scq7yw8YPWEwni9Rvy8R9pEFUCmscUSkRAu2LB9grPcr6L1NoNELBtiZZ58wm1cDrsgWForeaADkHnVmaqE6ULP";
const BIG_INPUT_FIRST_PUBLIC_KEY: &str = "ed25519:6Eb6wkNagkMg5EfZjo2AmStrUrxPKWLSHYDqVX7ofxtV";

const BIG_INPUT_SECOND_NONCE: &str = "7vX4hxEe9Hu5veUyivPWPxDHpYNsXzi7EG8bc05EIlA=";
const BIG_INPUT_SECOND_MESSAGE: &str = r#"{"signer_id":"foxboss.near","deadline":"2025-09-23T14:42:10.476Z","intents":[{"intent":"token_diff","diff":{"nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1":"-1000000000","nep141:wrap.near":"327579839172181597939094736"}}]}"#;
const BIG_INPUT_SECOND_SIGNATURE: &str = "ed25519:5Md212LF1YcemtoGCUPfB9sv1pijraj2VkKrFJscawXA7XXuVg6hvWTPcAvz2CuBH8Ns16Rmik1n7r9ySyyQJqWY";
const BIG_INPUT_SECOND_PUBLIC_KEY: &str = "ed25519:ESzYnUQTyVsNpk2u8ZHZ6q2MF8MHsuoKFRC4aqYAvcJD";

mod bindings {
    wasmtime::component::bindgen!({
        path: "benches/wit",
    });
}

mod codec_bindings {
    wasmtime::component::bindgen!({
        world: "format",
    });

    use exports::rvolosatovs::serde::reflect::{Guest, List, Value};
    use wasmtime::Store;

    macro_rules! impl_unwrap_value_primitive {
        ($rt:ident, $f:ident, $f_list:ident, $t:ty) => {
            #[allow(dead_code)]
            #[track_caller]
            pub fn $f(self) -> $t {
                let Value::$rt(v) = self else {
                    panic!("invalid value type");
                };
                v
            }

            #[allow(dead_code)]
            #[track_caller]
            pub fn $f_list(self) -> Vec<$t> {
                let Value::List(List::$rt(vs)) = self else {
                    panic!("invalid value list type");
                };
                vs
            }
        };
    }

    impl Value {
        impl_unwrap_value_primitive!(Bool, unwrap_bool, unwrap_list_bool, bool);
        impl_unwrap_value_primitive!(U8, unwrap_u8, unwrap_list_u8, u8);
        impl_unwrap_value_primitive!(U16, unwrap_u16, unwrap_list_u16, u16);
        impl_unwrap_value_primitive!(U32, unwrap_u32, unwrap_list_u32, u32);
        impl_unwrap_value_primitive!(U64, unwrap_u64, unwrap_list_u64, u64);
        impl_unwrap_value_primitive!(S8, unwrap_s8, unwrap_list_s8, i8);
        impl_unwrap_value_primitive!(S16, unwrap_s16, unwrap_list_s16, i16);
        impl_unwrap_value_primitive!(S32, unwrap_s32, unwrap_list_s32, i32);
        impl_unwrap_value_primitive!(S64, unwrap_s64, unwrap_list_s64, i64);
        impl_unwrap_value_primitive!(F32, unwrap_f32, unwrap_list_f32, f32);
        impl_unwrap_value_primitive!(F64, unwrap_f64, unwrap_list_f64, f64);
        impl_unwrap_value_primitive!(Char, unwrap_char, unwrap_list_char, char);
        impl_unwrap_value_primitive!(String, unwrap_string, unwrap_list_string, String);
        impl_unwrap_value_primitive!(Flags, unwrap_flags, unwrap_list_flags, u32);
        impl_unwrap_value_primitive!(Enum, unwrap_enum, unwrap_list_enum, u32);

        #[track_caller]
        pub fn unwrap_record<T>(self, store: &mut Store<T>, reflect: &Guest) -> Vec<Value> {
            let Value::Record(v) = self else {
                panic!("invalid value type");
            };
            reflect.record_value().call_into_value(store, v).unwrap()
        }

        #[track_caller]
        pub fn unwrap_tuple<T>(self, store: &mut Store<T>, reflect: &Guest) -> Vec<Value> {
            let Value::Tuple(v) = self else {
                panic!("invalid value type");
            };
            reflect.tuple_value().call_into_value(store, v).unwrap()
        }

        #[track_caller]
        pub fn unwrap_list(self) -> List {
            let Value::List(v) = self else {
                panic!("invalid value type");
            };
            v
        }
    }
}

struct Ctx<T> {
    input: Rc<[u8]>,
    state: T,
}

impl<T> Deref for Ctx<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T> DerefMut for Ctx<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

struct ModuleState {
    memory: ModuleExport,
}

impl<T> bindings::ComponentImports for Ctx<T> {
    fn input(&mut self) -> Vec<u8> {
        self.input.to_vec()
    }
}

fn unwrap_big_input_element_payload(
    fields: impl IntoIterator<Item = reflect::Value>,
) -> bindings::BigInputElementPayload {
    let mut fields = fields.into_iter();

    let nonce = fields.next().unwrap().unwrap_string();
    let message = fields.next().unwrap().unwrap_string();
    let recipient = fields.next().unwrap().unwrap_string();

    bindings::BigInputElementPayload {
        nonce,
        message,
        recipient,
    }
}

fn unwrap_big_input_element<T>(
    store: &mut Store<T>,
    reflect: &codec_bindings::exports::rvolosatovs::serde::reflect::Guest,
    fields: impl IntoIterator<Item = reflect::Value>,
) -> bindings::BigInputElement {
    let mut fields = fields.into_iter();

    let payload = fields.next().unwrap().unwrap_record(store, reflect);
    let payload = unwrap_big_input_element_payload(payload);

    let standard = fields.next().unwrap().unwrap_string();
    let signature = fields.next().unwrap().unwrap_string();
    let public_key = fields.next().unwrap().unwrap_string();

    bindings::BigInputElement {
        payload,
        standard,
        signature,
        public_key,
    }
}

fn unwrap_big_input<T>(
    mut store: &mut Store<T>,
    reflect: &codec_bindings::exports::rvolosatovs::serde::reflect::Guest,
    v: reflect::Value,
) -> bindings::BigInput {
    let fields = v.unwrap_record(store, reflect);
    let mut fields = fields.into_iter();

    let reflect::List::Record(elems) = fields.next().unwrap().unwrap_list() else {
        panic!("invalid list type");
    };
    let mut signed = Vec::with_capacity(elems.len());
    for v in elems {
        let fields = reflect
            .record_value()
            .call_into_value(&mut store, v)
            .unwrap();
        let el = unwrap_big_input_element(&mut store, reflect, fields);
        signed.push(el);
    }
    bindings::BigInput { signed }
}

fn bench_module(
    g: &mut BenchmarkGroup<impl Measurement>,
    wasm: &[u8],
    config: &wasmtime::Config,
) -> anyhow::Result<()> {
    fn input(mut caller: Caller<'_, Ctx<ModuleState>>, ptr: u64) {
        let memory = caller.data().memory;
        let Some(Extern::Memory(memory)) = caller.get_module_export(&memory) else {
            panic!()
        };
        let (memory, Ctx { input, .. }) = memory.data_and_store_mut(&mut caller);
        let ptr = ptr as usize;
        memory[ptr..ptr + input.len()].copy_from_slice(&input)
    }

    fn input_len(caller: Caller<'_, Ctx<ModuleState>>) -> u64 {
        caller.data().input.len() as _
    }

    fn make_setup<T: wasmtime::WasmParams>(
        engine: &Engine,
        pre: &wasmtime::InstancePre<Ctx<ModuleState>>,
        memory: ModuleExport,
    ) -> impl Fn(
        &ModuleExport,
        Rc<[u8]>,
    ) -> (
        wasmtime::TypedFunc<T, ()>,
        wasmtime::Memory,
        Store<Ctx<ModuleState>>,
    ) {
        move |export, input| {
            let mut store = Store::new(
                &engine,
                Ctx {
                    input,
                    state: ModuleState { memory },
                },
            );
            let instance = pre.instantiate(&mut store).unwrap();
            let Some(Extern::Func(f)) = instance.get_module_export(&mut store, export) else {
                panic!();
            };
            let Some(Extern::Memory(memory)) = instance.get_module_export(&mut store, &memory)
            else {
                panic!();
            };
            let f = f.typed(&store).unwrap();
            (f, memory, store)
        }
    }

    let engine = Engine::new(config)?;
    let module = Module::new(&engine, wasm)?;
    let mut linker = wasmtime::Linker::new(&engine);
    linker.func_wrap("env", "input", input)?;
    linker.func_wrap("env", "input_len", input_len)?;
    let memory = module.get_export_index("memory").unwrap();
    let noop = module.get_export_index("noop").unwrap();
    let run_small = module.get_export_index("run_small").unwrap();
    let run_small_bytes = module.get_export_index("run_small_bytes").unwrap();
    let run_big = module.get_export_index("run_big").unwrap();
    let run_big_bytes = module.get_export_index("run_big_bytes").unwrap();
    let pre = linker.instantiate_pre(&module)?;

    let setup = make_setup::<()>(&engine, &pre, memory);
    let setup_bytes =
        |export| make_setup::<(u32, u32)>(&engine, &pre, memory)(export, Rc::default());

    g.bench_function("noop", |b| {
        b.iter_batched(
            || {},
            |()| {
                let (f, _, store) = setup(&noop, Rc::default());
                f.call(store, ())
            },
            BatchSize::LargeInput,
        )
    });
    g.bench_with_input("small input", &Rc::from(SMALL_INPUT), |b, input| {
        b.iter_batched(
            || Rc::clone(input),
            |input| {
                let (f, _, store) = setup(&run_small, input);
                f.call(store, ())
            },
            BatchSize::LargeInput,
        )
    });
    g.bench_with_input(
        "small input byte args",
        &Rc::<[u8]>::from(SMALL_INPUT),
        |b, input| {
            b.iter_batched(
                || {},
                |()| {
                    let (f, memory, mut store) = setup_bytes(&run_small_bytes);
                    memory.data_mut(&mut store)[..input.len()].copy_from_slice(input);
                    f.call(store, (0, input.len() as _))
                },
                BatchSize::LargeInput,
            )
        },
    );
    g.bench_with_input("big input", &Rc::from(BIG_INPUT), |b, input| {
        b.iter_batched(
            || Rc::clone(input),
            |input| {
                let (f, _, store) = setup(&run_big, input);
                f.call(store, ())
            },
            BatchSize::LargeInput,
        )
    });
    g.bench_with_input(
        "big input byte args",
        &Rc::<[u8]>::from(BIG_INPUT),
        |b, input| {
            b.iter_batched(
                || {},
                |()| {
                    let (f, memory, mut store) = setup_bytes(&run_big_bytes);
                    memory.data_mut(&mut store)[..input.len()].copy_from_slice(input);
                    f.call(store, (0, input.len() as _))
                },
                BatchSize::LargeInput,
            )
        },
    );
    Ok(())
}

fn bench_component(
    g: &mut BenchmarkGroup<impl Measurement>,
    runner: &[u8],
    codec: &[u8],
    config: &wasmtime::Config,
) -> anyhow::Result<()> {
    let engine = Engine::new(config)?;

    let codec = Component::new(&engine, codec).context("failed to compile codec")?;
    let linker = component::Linker::new(&engine);
    let codec_pre = linker
        .instantiate_pre(&codec)
        .context("failed to pre-instantiate codec")?;
    let codec_pre =
        codec_bindings::FormatPre::new(codec_pre).context("failed to cast codec instance")?;

    let runner = Component::new(&engine, runner)?;
    let mut linker = component::Linker::new(&engine);
    bindings::Component::add_to_linker::<_, HasSelf<Ctx<()>>>(&mut linker, |cx| cx)?;
    let runner_pre = linker
        .instantiate_pre(&runner)
        .context("failed to pre-instantiate runner")?;
    let runner_pre =
        bindings::ComponentPre::new(runner_pre).context("failed to cast runner instance")?;

    let setup_runner = |input| {
        let mut store = Store::new(&engine, Ctx { input, state: () });
        let runner = runner_pre
            .instantiate(&mut store)
            .expect("failed to instantiate runner");
        (runner, store)
    };

    let setup_codec = || {
        let mut store = Store::new(&engine, ());
        let codec = codec_pre
            .instantiate(&mut store)
            .expect("failed to instantiate codec");
        (codec, store)
    };

    g.bench_function("noop", |b| {
        b.iter_batched(
            || {},
            |()| {
                let (runner, store) = setup_runner(Rc::default());
                runner.call_noop(store).unwrap();
            },
            BatchSize::LargeInput,
        );
    });
    g.bench_with_input("small input", &Rc::from(SMALL_INPUT), |b, input| {
        b.iter_batched(
            || Rc::clone(input),
            |input| {
                let (runner, store) = setup_runner(input);
                runner.call_run_small(store).unwrap();
            },
            BatchSize::LargeInput,
        );
    });
    g.bench_with_input(
        "small input byte args",
        &Rc::from(SMALL_INPUT),
        |b, input| {
            b.iter_batched(
                || {},
                |()| {
                    let (runner, store) = setup_runner(Rc::default());
                    runner.call_run_small_bytes(store, input).unwrap();
                },
                BatchSize::LargeInput,
            );
        },
    );

    g.bench_with_input(
        "small input typed args",
        &Rc::from(SMALL_INPUT),
        |b, input| {
            b.iter_batched(
                || {
                    let (codec, mut codec_store) = setup_codec();
                    let c_ty = codec
                        .rvolosatovs_serde_reflect()
                        .tuple_type()
                        .call_constructor(
                            &mut codec_store,
                            &[reflect::Type::U32, reflect::Type::U32, reflect::Type::U32],
                        )
                        .unwrap();
                    let ty = codec
                        .rvolosatovs_serde_reflect()
                        .record_type()
                        .call_constructor(
                            &mut codec_store,
                            &[
                                ("a".into(), reflect::Type::String),
                                ("b".into(), reflect::Type::U32),
                                ("c".into(), reflect::Type::Tuple(c_ty)),
                            ],
                        )
                        .unwrap();
                    (codec, codec_store, reflect::Type::Record(ty))
                },
                |(codec, mut codec_store, ty)| {
                    let (runner, runner_store) = setup_runner(Rc::default());
                    let v = codec
                        .rvolosatovs_serde_deserializer()
                        .call_from_list(&mut codec_store, input, ty)
                        .unwrap()
                        .unwrap();

                    let fields =
                        v.unwrap_record(&mut codec_store, codec.rvolosatovs_serde_reflect());
                    let mut fields = fields.into_iter();

                    let a = fields.next().unwrap().unwrap_string();
                    let b = fields.next().unwrap().unwrap_u32();
                    let c_values = fields
                        .next()
                        .unwrap()
                        .unwrap_tuple(&mut codec_store, codec.rvolosatovs_serde_reflect());
                    let mut c_values = c_values.into_iter();

                    let c0 = c_values.next().unwrap().unwrap_u32();
                    let c1 = c_values.next().unwrap().unwrap_u32();
                    let c2 = c_values.next().unwrap().unwrap_u32();

                    runner
                        .call_run_small_typed(
                            runner_store,
                            &bindings::SmallInput {
                                a,
                                b,
                                c: (c0, c1, c2),
                            },
                        )
                        .unwrap();
                },
                BatchSize::LargeInput,
            );
        },
    );
    g.bench_function("small input deserialized typed args", |b| {
        b.iter_batched(
            || bindings::SmallInput {
                a: "test".into(),
                b: 42,
                c: (0, 1, 2),
            },
            |input| {
                let (runner, store) = setup_runner(Rc::default());
                runner.call_run_small_typed(store, &input).unwrap();
            },
            BatchSize::LargeInput,
        );
    });
    g.bench_with_input("big input", &Rc::from(BIG_INPUT), |b, input| {
        b.iter_batched(
            || Rc::clone(input),
            |input| {
                let (runner, store) = setup_runner(input);
                runner.call_run_big(store).unwrap();
            },
            BatchSize::LargeInput,
        );
    });
    g.bench_with_input("big input byte args", &Rc::from(BIG_INPUT), |b, input| {
        b.iter_batched(
            || {},
            |()| {
                let (runner, store) = setup_runner(Rc::default());
                runner.call_run_big_bytes(store, input).unwrap();
            },
            BatchSize::LargeInput,
        );
    });
    g.bench_with_input("big input typed args", &Rc::from(BIG_INPUT), |b, input| {
        b.iter_batched(
            || {
                let (codec, mut codec_store) = setup_codec();

                let payload_ty = codec
                    .rvolosatovs_serde_reflect()
                    .record_type()
                    .call_constructor(
                        &mut codec_store,
                        &[
                            ("nonce".into(), reflect::Type::String),
                            ("message".into(), reflect::Type::String),
                            ("recipient".into(), reflect::Type::String),
                        ],
                    )
                    .unwrap();

                let signed_ty = codec
                    .rvolosatovs_serde_reflect()
                    .record_type()
                    .call_constructor(
                        &mut codec_store,
                        &[
                            ("payload".into(), reflect::Type::Record(payload_ty)),
                            ("standard".into(), reflect::Type::String),
                            ("signature".into(), reflect::Type::String),
                            ("public_key".into(), reflect::Type::String),
                        ],
                    )
                    .unwrap();

                let ty = codec
                    .rvolosatovs_serde_reflect()
                    .record_type()
                    .call_constructor(
                        &mut codec_store,
                        &[(
                            "signed".into(),
                            reflect::Type::List(reflect::ListType::Record(signed_ty)),
                        )],
                    )
                    .unwrap();
                (codec, codec_store, reflect::Type::Record(ty))
            },
            |(codec, mut codec_store, ty)| {
                let (runner, mut runner_store) = setup_runner(Rc::default());
                let v = codec
                    .rvolosatovs_serde_deserializer()
                    .call_from_list(&mut codec_store, input, ty)
                    .unwrap()
                    .unwrap();

                let input =
                    unwrap_big_input(&mut codec_store, codec.rvolosatovs_serde_reflect(), v);
                runner
                    .call_run_big_typed(&mut runner_store, &input)
                    .unwrap();
            },
            BatchSize::LargeInput,
        );
    });
    g.bench_function("big input deserialized typed args", |b| {
        b.iter_batched(
            || {
                let first = bindings::BigInputElement {
                    payload: bindings::BigInputElementPayload {
                        nonce: BIG_INPUT_FIRST_NONCE.into(),
                        message: BIG_INPUT_FIRST_MESSAGE.into(),
                        recipient: "intents.near".into(),
                    },
                    standard: "nep413".into(),
                    signature: BIG_INPUT_FIRST_SIGNATURE.into(),
                    public_key: BIG_INPUT_FIRST_PUBLIC_KEY.into(),
                };

                let second = bindings::BigInputElement {
                    payload: bindings::BigInputElementPayload {
                        nonce: BIG_INPUT_SECOND_NONCE.into(),
                        message: BIG_INPUT_SECOND_MESSAGE.into(),
                        recipient: "intents.near".into(),
                    },

                    standard: "nep413".into(),
                    signature: BIG_INPUT_SECOND_SIGNATURE.into(),
                    public_key: BIG_INPUT_SECOND_PUBLIC_KEY.into(),
                };
                bindings::BigInput {
                    signed: vec![first, second],
                }
            },
            |input| {
                let (runner, store) = setup_runner(Rc::default());
                runner.call_run_big_typed(store, &input).unwrap();
            },
            BatchSize::LargeInput,
        );
    });
    Ok(())
}

fn build(manifest: impl AsRef<OsStr>) -> anyhow::Result<()> {
    let res = Command::new(env!("CARGO"))
        .args([
            "build",
            "--workspace",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "--manifest-path",
        ])
        .arg(manifest)
        .status()
        .context("failed to build Wasm binaries")?;
    assert!(res.success());
    Ok(())
}

fn compose(runner: impl Into<Vec<u8>>, codec: impl Into<Vec<u8>>) -> anyhow::Result<Vec<u8>> {
    let mut graph = CompositionGraph::new();

    let runner = Package::from_bytes("runner", None, runner, graph.types_mut())?;
    let codec = Package::from_bytes("codec", None, codec, graph.types_mut())?;

    let runner = graph.register_package(runner)?;
    let codec = graph.register_package(codec)?;

    let runner = graph.instantiate(runner);
    let codec = graph.instantiate(codec);

    let deserializer =
        graph.alias_instance_export(codec, "rvolosatovs:serde/deserializer@0.1.0")?;
    let reflect = graph.alias_instance_export(codec, "rvolosatovs:serde/reflect@0.1.0")?;

    let noop = graph.alias_instance_export(runner, "noop")?;

    let run_small = graph.alias_instance_export(runner, "run-small")?;
    let run_big = graph.alias_instance_export(runner, "run-big")?;

    let run_small_bytes = graph.alias_instance_export(runner, "run-small-bytes")?;
    let run_big_bytes = graph.alias_instance_export(runner, "run-big-bytes")?;

    let run_small_typed = graph.alias_instance_export(runner, "run-small-typed")?;
    let run_big_typed = graph.alias_instance_export(runner, "run-big-typed")?;

    graph.set_instantiation_argument(
        runner,
        "rvolosatovs:serde/deserializer@0.1.0",
        deserializer,
    )?;
    graph.set_instantiation_argument(runner, "rvolosatovs:serde/reflect@0.1.0", reflect)?;
    graph.export(noop, "noop")?;
    graph.export(run_small, "run-small")?;
    graph.export(run_big, "run-big")?;
    graph.export(run_small_bytes, "run-small-bytes")?;
    graph.export(run_small_typed, "run-small-typed")?;
    graph.export(run_big_bytes, "run-big-bytes")?;
    graph.export(run_big_typed, "run-big-typed")?;

    let wasm = graph.encode(EncodeOptions::default())?;
    Ok(wasm)
}

// NOTE: This is adapted from current nearcore `main`
fn new_wasmtime_config(
    max_memory_pages: u32,
    max_tables_per_contract: u32,
    max_elements_per_contract_table: usize,
) -> wasmtime::Config {
    /// The maximum amount of concurrent calls this engine can handle.
    /// If this limit is reached, invocations will block until an execution slot is available.
    ///
    /// Wasmtime will use this value to pre-allocate and pool resources internally.
    /// Wasmtime defaults to `1_000`
    const MAX_CONCURRENCY: u32 = 1_000;

    /// Value used for [PoolingAllocationConfig::decommit_batch_size]
    ///
    /// Wasmtime defaults to `1`
    const DECOMMIT_BATCH_SIZE: usize = MAX_CONCURRENCY as usize / 2;

    /// Guest page size, in bytes
    const GUEST_PAGE_SIZE: usize = 1 << 16;

    fn guest_memory_size(pages: u32) -> Option<usize> {
        let pages = usize::try_from(pages).ok()?;
        pages.checked_mul(GUEST_PAGE_SIZE)
    }

    let max_memory_size = guest_memory_size(max_memory_pages).unwrap_or(usize::MAX);
    let max_tables = MAX_CONCURRENCY.saturating_mul(max_tables_per_contract);

    let mut pooling_config = wasmtime::PoolingAllocationConfig::default();
    pooling_config
        .decommit_batch_size(DECOMMIT_BATCH_SIZE)
        .max_memory_size(max_memory_size)
        .table_elements(max_elements_per_contract_table)
        .total_component_instances(MAX_CONCURRENCY)
        .total_core_instances(MAX_CONCURRENCY)
        .total_memories(MAX_CONCURRENCY)
        .total_tables(max_tables)
        .max_memories_per_module(1)
        .max_tables_per_module(max_tables_per_contract)
        .table_keep_resident(max_elements_per_contract_table);

    let mut config = wasmtime::Config::default();
    config
        .allocation_strategy(wasmtime::InstanceAllocationStrategy::Pooling(
            pooling_config,
        ))
        // From official documentation:
        // > Note that systems loading many modules may wish to disable this
        // > configuration option instead of leaving it on-by-default.
        // > Some platforms exhibit quadratic behavior when registering/unregistering
        // > unwinding information which can greatly slow down the module loading/unloading process.
        // https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.native_unwind_info
        .native_unwind_info(false)
        .wasm_backtrace(false)
        .wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Disable)
        // Enable copy-on-write heap images.
        .memory_init_cow(true)
        // Wasm stack metering is implemented by instrumentation, we don't want wasmtime to trap before that
        .max_wasm_stack(1024 * 1024 * 1024)
        // Enable the Cranelift optimizing compiler.
        .strategy(wasmtime::Strategy::Cranelift)
        // Enable signals-based traps. This is required to elide explicit bounds-checking.
        .signals_based_traps(true)
        // Configure linear memories such that explicit bounds-checking can be elided.
        .force_memory_init_memfd(true)
        .memory_guaranteed_dense_image_size(max_memory_size.try_into().unwrap_or(u64::MAX))
        .guard_before_linear_memory(false)
        .memory_guard_size(0)
        .memory_may_move(false)
        .memory_reservation(max_memory_size.try_into().unwrap_or(u64::MAX))
        .memory_reservation_for_growth(0)
        .compiler_inlining(true)
        .cranelift_nan_canonicalization(true)
        .wasm_wide_arithmetic(true);
    config
}

fn main() -> anyhow::Result<()> {
    let mut c = Criterion::default().configure_from_args();

    build(PathBuf::from_iter([
        env!("CARGO_MANIFEST_DIR"),
        "benches",
        "wasm",
        "Cargo.toml",
    ]))?;
    build(PathBuf::from_iter([
        env!("CARGO_MANIFEST_DIR"),
        "wasm-serde",
        "json",
        "Cargo.toml",
    ]))?;

    let module_bundle = fs::read(PathBuf::from_iter([
        env!("CARGO_MANIFEST_DIR"),
        "benches",
        "wasm",
        "target",
        "wasm32-unknown-unknown",
        "release",
        "module_bundle.wasm",
    ]))
    .context("failed to read `module_bundle.wasm`")?;

    let component_bundle = fs::read(PathBuf::from_iter([
        env!("CARGO_MANIFEST_DIR"),
        "benches",
        "wasm",
        "target",
        "wasm32-unknown-unknown",
        "release",
        "component_bundle.wasm",
    ]))
    .context("failed to read `component_bundle.wasm`")?;
    let mut component_bundle = ComponentEncoder::default().module(&component_bundle)?;
    let component_bundle = component_bundle.encode()?;

    let component_codec_import = fs::read(PathBuf::from_iter([
        env!("CARGO_MANIFEST_DIR"),
        "benches",
        "wasm",
        "target",
        "wasm32-unknown-unknown",
        "release",
        "component_codec_import.wasm",
    ]))
    .context("failed to read `component_codec_import.wasm`")?;
    let mut component_codec_import = ComponentEncoder::default().module(&component_codec_import)?;
    let component_codec_import = component_codec_import.encode()?;

    let codec = fs::read(PathBuf::from_iter([
        env!("CARGO_MANIFEST_DIR"),
        "wasm-serde",
        "target",
        "wasm32-unknown-unknown",
        "release",
        "wasm_serde_json.wasm",
    ]))
    .context("failed to read `wasm_serde_json.wasm`")?;
    let mut codec = ComponentEncoder::default().module(&codec)?;
    let codec = codec.encode()?;

    let component_composed = compose(component_codec_import.as_slice(), codec.as_slice())?;

    let config = new_wasmtime_config(2_048, 1, 10_000);

    {
        let mut g = c.benchmark_group("module bundling serde_json");
        bench_module(&mut g, &module_bundle, &config)?;
        g.finish();
    }
    {
        let mut g = c.benchmark_group("component bundling serde_json");
        bench_component(&mut g, &component_bundle, &codec, &config)?;
        g.finish();
    }
    {
        let mut g = c.benchmark_group("component composed with codec");
        bench_component(&mut g, &component_composed, &codec, &config)?;
        g.finish();
    }
    c.final_summary();
    Ok(())
}
