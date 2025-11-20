use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

use anyhow::Context as _;
use criterion::measurement::Measurement;
use criterion::{BenchmarkGroup, Criterion};
use wac_graph::types::Package;
use wac_graph::{CompositionGraph, EncodeOptions};
use wasmtime::component::{Component, HasSelf, Resource, ResourceTable};
use wasmtime::{Caller, Engine, Extern, Module, ModuleExport, Store, component};
use wit_component::ComponentEncoder;

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

fn bench_module(
    g: &mut BenchmarkGroup<impl Measurement>,
    wasm: &[u8],
    config: &wasmtime::Config,
) -> anyhow::Result<()> {
    struct Ctx {
        input: Rc<[u8]>,
        memory: ModuleExport,
    }

    fn input(mut caller: Caller<'_, Ctx>, ptr: u64) {
        let memory = caller.data().memory;
        let Some(Extern::Memory(memory)) = caller.get_module_export(&memory) else {
            panic!()
        };
        let (memory, Ctx { input, .. }) = memory.data_and_store_mut(&mut caller);
        let ptr = ptr as usize;
        memory[ptr..ptr + input.len()].copy_from_slice(&input)
    }

    fn input_len(caller: Caller<'_, Ctx>) -> u64 {
        caller.data().input.len() as _
    }

    let engine = Engine::new(config)?;
    let module = Module::new(&engine, wasm)?;
    let mut linker = wasmtime::Linker::new(&engine);
    linker.func_wrap("env", "input", input)?;
    linker.func_wrap("env", "input_len", input_len)?;
    let memory = module.get_export_index("memory").unwrap();
    let noop = module.get_export_index("noop").unwrap();
    let run_small = module.get_export_index("run_small").unwrap();
    let run_big = module.get_export_index("run_big").unwrap();
    let pre = linker.instantiate_pre(&module)?;
    g.bench_function("noop", |b| {
        b.iter_with_large_drop(|| {
            let mut store = Store::new(
                &engine,
                Ctx {
                    input: Rc::default(),
                    memory,
                },
            );
            let instance = pre.instantiate(&mut store).unwrap();
            let Some(Extern::Func(f)) = instance.get_module_export(&mut store, &noop) else {
                panic!();
            };
            let f = f.typed::<(), ()>(&store).unwrap();
            f.call(store, ())
        });
    });
    g.bench_with_input("small input", &Rc::from(SMALL_INPUT), |b, input| {
        b.iter_with_large_drop(|| {
            let input = Rc::clone(input);
            let mut store = Store::new(&engine, Ctx { input, memory });
            let instance = pre.instantiate(&mut store).unwrap();
            let Some(Extern::Func(f)) = instance.get_module_export(&mut store, &run_small) else {
                panic!();
            };
            let f = f.typed::<(), ()>(&store).unwrap();
            f.call(store, ())
        });
    });
    g.bench_with_input("big input", &Rc::from(BIG_INPUT), |b, input| {
        b.iter_with_large_drop(|| {
            let input = Rc::clone(input);
            let mut store = Store::new(&engine, Ctx { input, memory });
            let instance = pre.instantiate(&mut store).unwrap();
            let Some(Extern::Func(f)) = instance.get_module_export(&mut store, &run_big) else {
                panic!();
            };
            let f = f.typed::<(), ()>(&store).unwrap();
            f.call(store, ())
        });
    });
    Ok(())
}

fn bench_component(
    g: &mut BenchmarkGroup<impl Measurement>,
    runner: &[u8],
    codec: &[u8],
    config: &wasmtime::Config,
) -> anyhow::Result<()> {
    mod bindings {
        wasmtime::component::bindgen!({
            path: "benches/wit",
            with: {
                "rvolosatovs:serde/deserializer@0.1.0/deserializer": wasmtime::component::ResourceAny,
                "rvolosatovs:serde/deserializer@0.1.0/error": wasmtime::component::ResourceAny,
                "rvolosatovs:serde/deserializer@0.1.0/list-deserializer": wasmtime::component::ResourceAny,
                "rvolosatovs:serde/deserializer@0.1.0/record-deserializer": wasmtime::component::ResourceAny,
                "rvolosatovs:serde/deserializer@0.1.0/tuple-deserializer": wasmtime::component::ResourceAny,
            }
        });
    }

    mod codec_bindings {
        wasmtime::component::bindgen!({
            path: "benches/wit",
            inline: "
                package near-cm:codec;

                world component {
                    export rvolosatovs:serde/deserializer@0.1.0;
                }
",
        });
    }

    use bindings::rvolosatovs::serde::deserializer::{
        Deserializer, Error, ListDeserializer, RecordDeserializer, TupleDeserializer,
    };

    struct Ctx {
        input: Rc<[u8]>,
        table: ResourceTable,
        codec: codec_bindings::Component,
        codec_store: Store<()>,
    }

    impl bindings::ComponentImports for Ctx {
        fn input(&mut self) -> Vec<u8> {
            self.input.to_vec()
        }
    }

    impl bindings::rvolosatovs::serde::deserializer::HostError for Ctx {
        fn to_string(&mut self, err: Resource<Error>) -> String {
            let err = self.table.delete(err).unwrap();
            self.codec
                .rvolosatovs_serde_deserializer()
                .error()
                .call_to_string(&mut self.codec_store, err)
                .unwrap()
        }

        fn drop(&mut self, err: Resource<Error>) -> wasmtime::Result<()> {
            self.table.delete(err)?;
            Ok(())
        }
    }
    #[expect(unused)]
    impl bindings::rvolosatovs::serde::deserializer::HostDeserializer for Ctx {
        fn from_list(&mut self, buf: Vec<u8>) -> Resource<Deserializer> {
            let de = self
                .codec
                .rvolosatovs_serde_deserializer()
                .deserializer()
                .call_from_list(&mut self.codec_store, &buf)
                .unwrap();
            self.table.push(de).unwrap()
        }

        fn deserialize_bool(
            &mut self,
            de: Resource<Deserializer>,
        ) -> Result<bool, Resource<Error>> {
            todo!()
        }

        fn deserialize_u8(&mut self, de: Resource<Deserializer>) -> Result<u8, Resource<Error>> {
            todo!()
        }

        fn deserialize_s8(&mut self, de: Resource<Deserializer>) -> Result<i8, Resource<Error>> {
            todo!()
        }

        fn deserialize_u16(&mut self, de: Resource<Deserializer>) -> Result<u16, Resource<Error>> {
            todo!()
        }

        fn deserialize_s16(&mut self, de: Resource<Deserializer>) -> Result<i16, Resource<Error>> {
            todo!()
        }

        fn deserialize_u32(&mut self, de: Resource<Deserializer>) -> Result<u32, Resource<Error>> {
            let de = self.table.delete(de).unwrap();
            let v = self
                .codec
                .rvolosatovs_serde_deserializer()
                .deserializer()
                .call_deserialize_u32(&mut self.codec_store, de)
                .unwrap()
                .unwrap();
            Ok(v)
        }

        fn deserialize_s32(&mut self, de: Resource<Deserializer>) -> Result<i32, Resource<Error>> {
            todo!()
        }

        fn deserialize_u64(&mut self, de: Resource<Deserializer>) -> Result<u64, Resource<Error>> {
            todo!()
        }

        fn deserialize_s64(&mut self, de: Resource<Deserializer>) -> Result<i64, Resource<Error>> {
            todo!()
        }

        fn deserialize_f32(&mut self, de: Resource<Deserializer>) -> Result<f32, Resource<Error>> {
            todo!()
        }

        fn deserialize_f64(&mut self, de: Resource<Deserializer>) -> Result<f64, Resource<Error>> {
            todo!()
        }

        fn deserialize_char(
            &mut self,
            de: Resource<Deserializer>,
        ) -> Result<char, Resource<Error>> {
            todo!()
        }

        fn deserialize_bytes(
            &mut self,
            de: Resource<Deserializer>,
        ) -> Result<Vec<u8>, Resource<Error>> {
            todo!()
        }

        fn deserialize_string(
            &mut self,
            de: Resource<Deserializer>,
        ) -> Result<String, Resource<Error>> {
            let de = self.table.delete(de).unwrap();
            let v = self
                .codec
                .rvolosatovs_serde_deserializer()
                .deserializer()
                .call_deserialize_string(&mut self.codec_store, de)
                .unwrap()
                .unwrap();
            Ok(v)
        }

        fn deserialize_record(
            &mut self,
            de: Resource<Deserializer>,
            fields: Vec<String>,
        ) -> Result<(u32, Resource<Deserializer>, Resource<RecordDeserializer>), Resource<Error>>
        {
            let de = self.table.delete(de).unwrap();
            let (idx, de, next) = self
                .codec
                .rvolosatovs_serde_deserializer()
                .deserializer()
                .call_deserialize_record(&mut self.codec_store, de, &fields)
                .unwrap()
                .unwrap();
            let de = self.table.push(de).unwrap();
            let next = self.table.push(next).unwrap();
            Ok((idx, de, next))
        }

        fn deserialize_variant(
            &mut self,
            de: Resource<Deserializer>,
            cases: Vec<(String, bool)>,
        ) -> Result<(u32, Resource<Deserializer>), Resource<Error>> {
            todo!()
        }

        fn deserialize_list(
            &mut self,
            de: Resource<Deserializer>,
        ) -> Result<Resource<ListDeserializer>, Resource<Error>> {
            let de = self.table.delete(de).unwrap();
            let de = self
                .codec
                .rvolosatovs_serde_deserializer()
                .deserializer()
                .call_deserialize_list(&mut self.codec_store, de)
                .unwrap()
                .unwrap();
            let de = self.table.push(de).unwrap();
            Ok(de)
        }

        fn deserialize_tuple(
            &mut self,
            de: Resource<Deserializer>,
            n: u32,
        ) -> Result<(Resource<Deserializer>, Resource<TupleDeserializer>), Resource<Error>>
        {
            let de = self.table.delete(de).unwrap();
            let (de, next) = self
                .codec
                .rvolosatovs_serde_deserializer()
                .deserializer()
                .call_deserialize_tuple(&mut self.codec_store, de, n)
                .unwrap()
                .unwrap();
            let de = self.table.push(de).unwrap();
            let next = self.table.push(next).unwrap();
            Ok((de, next))
        }

        fn deserialize_flags(
            &mut self,
            de: Resource<Deserializer>,
            cases: Vec<String>,
        ) -> Result<u32, Resource<Error>> {
            todo!()
        }

        fn deserialize_enum(
            &mut self,
            de: Resource<Deserializer>,
            cases: Vec<String>,
        ) -> Result<u32, Resource<Error>> {
            todo!()
        }

        fn deserialize_option(
            &mut self,
            de: Resource<Deserializer>,
        ) -> Result<Option<Resource<Deserializer>>, Resource<Error>> {
            todo!()
        }

        fn deserialize_result(
            &mut self,
            de: Resource<Deserializer>,
            ok: bool,
            err: bool,
        ) -> Result<Result<Resource<Deserializer>, Resource<Deserializer>>, Resource<Error>>
        {
            todo!()
        }

        fn drop(&mut self, de: Resource<Deserializer>) -> wasmtime::Result<()> {
            self.table.delete(de)?;
            Ok(())
        }
    }
    impl bindings::rvolosatovs::serde::deserializer::HostTupleDeserializer for Ctx {
        fn next(
            &mut self,
            de: Resource<TupleDeserializer>,
        ) -> (Resource<Deserializer>, Resource<TupleDeserializer>) {
            let de = self.table.delete(de).unwrap();
            let (de, next) = self
                .codec
                .rvolosatovs_serde_deserializer()
                .tuple_deserializer()
                .call_next(&mut self.codec_store, de)
                .unwrap();
            let de = self.table.push(de).unwrap();
            let next = self.table.push(next).unwrap();
            (de, next)
        }

        fn drop(&mut self, de: Resource<TupleDeserializer>) -> wasmtime::Result<()> {
            self.table.delete(de)?;
            Ok(())
        }
    }
    impl bindings::rvolosatovs::serde::deserializer::HostRecordDeserializer for Ctx {
        fn next(
            &mut self,
            de: Resource<RecordDeserializer>,
        ) -> (u32, Resource<Deserializer>, Resource<RecordDeserializer>) {
            let de = self.table.delete(de).unwrap();
            let (idx, de, next) = self
                .codec
                .rvolosatovs_serde_deserializer()
                .record_deserializer()
                .call_next(&mut self.codec_store, de)
                .unwrap();
            let de = self.table.push(de).unwrap();
            let next = self.table.push(next).unwrap();
            (idx, de, next)
        }

        fn drop(&mut self, de: Resource<RecordDeserializer>) -> wasmtime::Result<()> {
            self.table.delete(de)?;
            Ok(())
        }
    }
    impl bindings::rvolosatovs::serde::deserializer::HostListDeserializer for Ctx {
        fn next(
            &mut self,
            de: Resource<ListDeserializer>,
        ) -> Option<(Resource<Deserializer>, Resource<ListDeserializer>)> {
            let de = self.table.delete(de).unwrap();
            let (de, next) = self
                .codec
                .rvolosatovs_serde_deserializer()
                .list_deserializer()
                .call_next(&mut self.codec_store, de)
                .unwrap()?;
            let de = self.table.push(de).unwrap();
            let next = self.table.push(next).unwrap();
            Some((de, next))
        }

        fn drop(&mut self, de: Resource<ListDeserializer>) -> wasmtime::Result<()> {
            self.table.delete(de)?;
            Ok(())
        }
    }
    impl bindings::rvolosatovs::serde::deserializer::Host for Ctx {}

    let engine = Engine::new(config)?;

    let codec = Component::new(&engine, codec)?;
    let linker = component::Linker::new(&engine);
    let codec_pre = linker.instantiate_pre(&codec)?;
    let codec_pre = codec_bindings::ComponentPre::new(codec_pre)?;

    let runner = Component::new(&engine, runner)?;
    let mut linker = component::Linker::new(&engine);
    bindings::Component::add_to_linker::<_, HasSelf<Ctx>>(&mut linker, |cx| cx)?;
    let runner_pre = linker.instantiate_pre(&runner)?;
    let runner_pre = bindings::ComponentPre::new(runner_pre)?;
    g.bench_function("noop", |b| {
        b.iter_with_large_drop(|| {
            let mut codec_store = Store::new(&engine, ());
            let codec = codec_pre.instantiate(&mut codec_store).unwrap();
            let mut store = Store::new(
                &engine,
                Ctx {
                    input: Rc::default(),
                    table: ResourceTable::default(),
                    codec,
                    codec_store,
                },
            );
            let runner = runner_pre.instantiate(&mut store).unwrap();
            runner.call_noop(store).unwrap();
        });
    });
    g.bench_with_input("small input", &Rc::from(SMALL_INPUT), |b, input| {
        b.iter_with_large_drop(|| {
            let mut codec_store = Store::new(&engine, ());
            let codec = codec_pre.instantiate(&mut codec_store).unwrap();
            let input = Rc::clone(input);
            let mut store = Store::new(
                &engine,
                Ctx {
                    input,
                    table: ResourceTable::default(),
                    codec,
                    codec_store,
                },
            );
            let runner = runner_pre.instantiate(&mut store).unwrap();
            runner.call_run_small(store).unwrap();
        });
    });
    g.bench_with_input(
        "small input byte args",
        &Rc::from(SMALL_INPUT),
        |b, input| {
            b.iter_with_large_drop(|| {
                let mut codec_store = Store::new(&engine, ());
                let codec = codec_pre.instantiate(&mut codec_store).unwrap();
                let mut store = Store::new(
                    &engine,
                    Ctx {
                        input: Rc::default(),
                        table: ResourceTable::default(),
                        codec,
                        codec_store,
                    },
                );
                let instance = runner_pre.instantiate(&mut store).unwrap();
                instance.call_run_small_bytes(store, input).unwrap();
            });
        },
    );
    g.bench_with_input("big input", &Rc::from(BIG_INPUT), |b, input| {
        b.iter_with_large_drop(|| {
            let mut codec_store = Store::new(&engine, ());
            let codec = codec_pre.instantiate(&mut codec_store).unwrap();
            let input = Rc::clone(input);
            let mut store = Store::new(
                &engine,
                Ctx {
                    input,
                    table: ResourceTable::default(),
                    codec,
                    codec_store,
                },
            );
            let runner = runner_pre.instantiate(&mut store).unwrap();
            runner.call_run_big(store).unwrap();
        });
    });
    g.bench_with_input("big input byte args", &Rc::from(BIG_INPUT), |b, input| {
        b.iter_with_large_drop(|| {
            let mut codec_store = Store::new(&engine, ());
            let codec = codec_pre.instantiate(&mut codec_store).unwrap();
            let mut store = Store::new(
                &engine,
                Ctx {
                    input: Rc::default(),
                    table: ResourceTable::default(),
                    codec,
                    codec_store,
                },
            );
            let runner = runner_pre.instantiate(&mut store).unwrap();
            runner.call_run_big_bytes(store, input).unwrap();
        });
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
    let noop = graph.alias_instance_export(runner, "noop")?;
    let run_small = graph.alias_instance_export(runner, "run-small")?;
    let run_big = graph.alias_instance_export(runner, "run-big")?;
    let run_small_bytes = graph.alias_instance_export(runner, "run-small-bytes")?;
    let run_big_bytes = graph.alias_instance_export(runner, "run-big-bytes")?;

    graph.set_instantiation_argument(
        runner,
        "rvolosatovs:serde/deserializer@0.1.0",
        deserializer,
    )?;
    graph.export(noop, "noop")?;
    graph.export(run_small, "run-small")?;
    graph.export(run_big, "run-big")?;
    graph.export(run_small_bytes, "run-small-bytes")?;
    graph.export(run_big_bytes, "run-big-bytes")?;

    let wasm = graph.encode(EncodeOptions::default())?;
    Ok(wasm)
}

// NOTE: This is adapted from current nearcore `main`
fn new_wasmtime_config(
    _max_memory_pages: u32,
    _max_tables_per_contract: u32,
    _max_elements_per_contract_table: usize,
) -> wasmtime::Config {
    // /// The maximum amount of concurrent calls this engine can handle.
    // /// If this limit is reached, invocations will block until an execution slot is available.
    // ///
    // /// Wasmtime will use this value to pre-allocate and pool resources internally.
    // /// Wasmtime defaults to `1_000`
    // const MAX_CONCURRENCY: u32 = 1_000;

    // /// Value used for [PoolingAllocationConfig::decommit_batch_size]
    // ///
    // /// Wasmtime defaults to `1`
    // const DECOMMIT_BATCH_SIZE: usize = MAX_CONCURRENCY as usize / 2;

    // /// Guest page size, in bytes
    // const GUEST_PAGE_SIZE: usize = 1 << 16;

    // fn guest_memory_size(pages: u32) -> Option<usize> {
    //     let pages = usize::try_from(pages).ok()?;
    //     pages.checked_mul(GUEST_PAGE_SIZE)
    // }

    // let max_memory_size = guest_memory_size(max_memory_pages).unwrap_or(usize::MAX);
    // let max_tables = MAX_CONCURRENCY.saturating_mul(max_tables_per_contract);

    let pooling_config = wasmtime::PoolingAllocationConfig::default();
    // pooling_config
    //     .decommit_batch_size(DECOMMIT_BATCH_SIZE)
    //     .max_memory_size(max_memory_size)
    //     .table_elements(max_elements_per_contract_table)
    //     .total_component_instances(MAX_CONCURRENCY)
    //     .total_core_instances(MAX_CONCURRENCY)
    //     .total_memories(MAX_CONCURRENCY)
    //     .total_tables(max_tables)
    //     .max_memories_per_module(1)
    //     .max_tables_per_module(max_tables_per_contract)
    //     .table_keep_resident(max_elements_per_contract_table);

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
        // .native_unwind_info(false)
        // .wasm_backtrace(false)
        // .wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Disable)
        // // Enable copy-on-write heap images.
        // .memory_init_cow(true)
        // // Wasm stack metering is implemented by instrumentation, we don't want wasmtime to trap before that
        // .max_wasm_stack(1024 * 1024 * 1024)
        // // Enable the Cranelift optimizing compiler.
        // .strategy(wasmtime::Strategy::Cranelift)
        // // Enable signals-based traps. This is required to elide explicit bounds-checking.
        // .signals_based_traps(true)
        // // Configure linear memories such that explicit bounds-checking can be elided.
        // .force_memory_init_memfd(true)
        // .memory_guaranteed_dense_image_size(max_memory_size.try_into().unwrap_or(u64::MAX))
        // .guard_before_linear_memory(false)
        // .memory_guard_size(0)
        // .memory_may_move(false)
        // .memory_reservation(max_memory_size.try_into().unwrap_or(u64::MAX))
        // .memory_reservation_for_growth(0)
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
        "json",
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
    {
        let mut g = c.benchmark_group("component composed with codec at runtime");
        bench_component(&mut g, &component_codec_import, &codec, &config)?;
        g.finish();
    }
    c.final_summary();
    Ok(())
}
