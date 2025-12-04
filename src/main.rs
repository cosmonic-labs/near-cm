mod bindings {
    wasmtime::component::bindgen!({
        world: "format",
        exports: {
            default: async,
        }
    });
}

use core::iter::zip;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context as _, bail, ensure};
use bytes::{Buf, Bytes};
use http_body_util::BodyExt as _;
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use tokio::fs;
use tokio::net::TcpListener;
use url::Url;
use wasmtime::component::{Component, InstancePre, Linker, Type, Val, types};
use wasmtime::{Engine, Store};
use wit_component::ComponentEncoder;

use bindings::exports::rvolosatovs::serde::reflect;

pub struct Error;

async fn unwrap_val(
    mut store: &mut Store<()>,
    v: reflect::Value,
    instance: &reflect::Guest,
    ty: Type,
) -> wasmtime::Result<Val> {
    match (v, ty) {
        (reflect::Value::Bool(v), Type::Bool) => Ok(Val::Bool(v)),
        (reflect::Value::S8(v), Type::S8) => Ok(Val::S8(v)),
        (reflect::Value::U8(v), Type::U8) => Ok(Val::U8(v)),
        (reflect::Value::U16(v), Type::U16) => Ok(Val::U16(v)),
        (reflect::Value::S16(v), Type::S16) => Ok(Val::S16(v)),
        (reflect::Value::U32(v), Type::U32) => Ok(Val::U32(v)),
        (reflect::Value::S32(v), Type::S32) => Ok(Val::S32(v)),
        (reflect::Value::U64(v), Type::U64) => Ok(Val::U64(v)),
        (reflect::Value::S64(v), Type::S64) => Ok(Val::S64(v)),
        (reflect::Value::F32(v), Type::Float32) => Ok(Val::Float32(v)),
        (reflect::Value::F64(v), Type::Float64) => Ok(Val::Float64(v)),
        (reflect::Value::Char(v), Type::Char) => Ok(Val::Char(v)),
        (reflect::Value::String(v), Type::String) => Ok(Val::String(v)),
        (reflect::Value::Record(v), Type::Record(ty)) => {
            let values = instance
                .record_value()
                .call_into_value(&mut store, v)
                .await?;
            ensure!(values.len() == ty.fields().len());

            let mut fields = Vec::with_capacity(values.len());
            for (types::Field { name, ty }, v) in zip(ty.fields(), values) {
                let v = Box::pin(unwrap_val(store, v, instance, ty))
                    .await
                    .with_context(|| format!("failed to unwrap record field `{name}`"))?;
                fields.push((name.into(), v));
            }
            Ok(Val::Record(fields))
        }
        #[expect(unused, reason = "incomplete")]
        (reflect::Value::Variant(v), Type::Variant(ty)) => todo!(),
        #[expect(unused, reason = "incomplete")]
        (reflect::Value::List(v), Type::List(ty)) => todo!(),
        (reflect::Value::Tuple(v), Type::Tuple(ty)) => {
            let values = instance
                .tuple_value()
                .call_into_value(&mut store, v)
                .await?;
            ensure!(values.len() == ty.types().len());

            let mut elems = Vec::with_capacity(values.len());
            for ((i, ty), v) in zip(ty.types().enumerate(), values) {
                let v = Box::pin(unwrap_val(store, v, instance, ty))
                    .await
                    .with_context(|| format!("failed to unwrap tuple element `{i}`"))?;
                elems.push(v);
            }
            Ok(Val::Tuple(elems))
        }
        #[expect(unused, reason = "incomplete")]
        (reflect::Value::Flags(v), Type::Flags(ty)) => todo!(),
        #[expect(unused, reason = "incomplete")]
        (reflect::Value::Enum(v), Type::Enum(ty)) => todo!(),
        #[expect(unused, reason = "incomplete")]
        (reflect::Value::Option(v), Type::Option(ty)) => todo!(),
        #[expect(unused, reason = "incomplete")]
        (reflect::Value::Result(v), Type::Result(ty)) => todo!(),
        _ => bail!("type mismatch"),
    }
}

async fn make_reflect_ty(
    store: &mut Store<()>,
    instance: &reflect::Guest,
    ty: Type,
) -> wasmtime::Result<reflect::Type> {
    match ty {
        Type::Bool => Ok(reflect::Type::Bool),
        Type::S8 => Ok(reflect::Type::S8),
        Type::U8 => Ok(reflect::Type::U8),
        Type::S16 => Ok(reflect::Type::S16),
        Type::U16 => Ok(reflect::Type::U16),
        Type::S32 => Ok(reflect::Type::S32),
        Type::U32 => Ok(reflect::Type::U32),
        Type::S64 => Ok(reflect::Type::S64),
        Type::U64 => Ok(reflect::Type::U64),
        Type::Float32 => Ok(reflect::Type::F32),
        Type::Float64 => Ok(reflect::Type::F64),
        Type::Char => Ok(reflect::Type::Char),
        Type::String => Ok(reflect::Type::String),
        Type::List(ty) => {
            match ty.ty() {
                Type::Bool => Ok(reflect::Type::List(reflect::ListType::Bool)),
                Type::S8 => Ok(reflect::Type::List(reflect::ListType::S8)),
                Type::U8 => Ok(reflect::Type::List(reflect::ListType::U8)),
                Type::S16 => Ok(reflect::Type::List(reflect::ListType::S16)),
                Type::U16 => Ok(reflect::Type::List(reflect::ListType::U16)),
                Type::S32 => Ok(reflect::Type::List(reflect::ListType::S32)),
                Type::U32 => Ok(reflect::Type::List(reflect::ListType::U32)),
                Type::S64 => Ok(reflect::Type::List(reflect::ListType::S64)),
                Type::U64 => Ok(reflect::Type::List(reflect::ListType::U64)),
                Type::Float32 => Ok(reflect::Type::List(reflect::ListType::F32)),
                Type::Float64 => Ok(reflect::Type::List(reflect::ListType::F64)),
                Type::Char => Ok(reflect::Type::List(reflect::ListType::Char)),
                Type::String => Ok(reflect::Type::List(reflect::ListType::String)),
                //Type::List(list) => Ok(reflect::Type::List(reflect::ListType::List)),
                //Type::Record(record) => Ok(reflect::Type::List(reflect::ListType::Record)),
                //Type::Tuple(tuple) => Ok(reflect::Type::List(reflect::ListType::Tuple)),
                //Type::Variant(variant) => Ok(reflect::Type::List(reflect::ListType::Variant)),
                //Type::Enum(_) => Ok(reflect::Type::List(reflect::ListType::Enum)),
                //Type::Option(option_type) => Ok(reflect::Type::List(reflect::ListType::Option)),
                //Type::Result(result_type) => Ok(reflect::Type::List(reflect::ListType::Result)),
                //Type::Flags(flags) => Ok(reflect::Type::List(reflect::ListType::Flags)),
                Type::Own(..) | Type::Borrow(..) => bail!("resources not supported"),
                Type::Future(..) => bail!("futures not supported"),
                Type::Stream(..) => bail!("streams not supported"),
                Type::ErrorContext => bail!("error context not supported"),
                _ => todo!(),
            }
        }
        Type::Record(ty) => {
            let mut fields = Vec::with_capacity(ty.fields().len());
            for types::Field { name, ty } in ty.fields() {
                let ty = Box::pin(make_reflect_ty(store, instance, ty)).await?;
                fields.push((name.into(), ty))
            }
            let ty = instance
                .record_type()
                .call_constructor(store, &fields)
                .await?;
            Ok(reflect::Type::Record(ty))
        }
        Type::Tuple(ty) => {
            let mut tys = Vec::with_capacity(ty.types().len());
            for ty in ty.types() {
                let ty = Box::pin(make_reflect_ty(store, instance, ty)).await?;
                tys.push(ty)
            }
            let ty = instance.tuple_type().call_constructor(store, &tys).await?;
            Ok(reflect::Type::Tuple(ty))
        }
        #[expect(unused, reason = "incomplete")]
        Type::Variant(ty) => todo!(),
        #[expect(unused, reason = "incomplete")]
        Type::Enum(ty) => todo!(),
        #[expect(unused, reason = "incomplete")]
        Type::Option(ty) => todo!(),
        #[expect(unused, reason = "incomplete")]
        Type::Result(ty) => todo!(),
        #[expect(unused, reason = "incomplete")]
        Type::Flags(ty) => todo!(),
        #[expect(unused, reason = "incomplete")]
        Type::Own(ty) => todo!(),
        #[expect(unused, reason = "incomplete")]
        Type::Borrow(ty) => todo!(),
        #[expect(unused, reason = "incomplete")]
        Type::Future(ty) => todo!(),
        #[expect(unused, reason = "incomplete")]
        Type::Stream(ty) => todo!(),
        Type::ErrorContext => todo!(),
    }
}

async fn deserialize_params(
    mut store: &mut Store<()>,
    instance: &bindings::Format,
    ty: &types::ComponentFunc,
    body: hyper::body::Incoming,
) -> wasmtime::Result<Vec<Val>> {
    let tys = ty.params();
    let num_params = tys.len();
    if num_params == 0 {
        let body = body.collect().await?;
        ensure!(body.to_bytes().is_empty());
        return Ok(Vec::default());
    }

    let mut reflect_tys = Vec::with_capacity(ty.params().len());
    for (_, ty) in ty.params() {
        let ty = make_reflect_ty(store, instance.rvolosatovs_serde_reflect(), ty).await?;
        reflect_tys.push(ty);
    }
    let reflect_ty = instance
        .rvolosatovs_serde_reflect()
        .tuple_type()
        .call_constructor(&mut store, &reflect_tys)
        .await?;

    let body = body.collect().await?;
    let values = match instance
        .rvolosatovs_serde_deserializer()
        .call_from_list(
            &mut store,
            &body.to_bytes(),
            reflect::Type::Tuple(reflect_ty),
        )
        .await?
    {
        Ok(value) => value,
        Err(err) => {
            let err = instance
                .rvolosatovs_serde_deserializer()
                .error()
                .call_to_string(store, err)
                .await?;
            bail!(err)
        }
    };
    let reflect::Value::Tuple(values) = values else {
        bail!("deserialized value is not a tuple");
    };
    let values = instance
        .rvolosatovs_serde_reflect()
        .tuple_value()
        .call_into_value(&mut store, values)
        .await?;
    ensure!(values.len() == num_params);

    let mut params = Vec::with_capacity(num_params);
    for ((name, ty), v) in zip(tys, values) {
        let v = unwrap_val(store, v, instance.rvolosatovs_serde_reflect(), ty)
            .await
            .with_context(|| format!("failed to unwrap param `{name}`"))?;
        params.push(v);
    }
    Ok(params)
}

struct Contract {
    pre: InstancePre<()>,
    ty: types::Component,
}

fn build_http_response<T>(
    code: http::StatusCode,
    body: impl Into<T>,
) -> anyhow::Result<http::Response<http_body_util::Full<T>>>
where
    T: Buf + Sync + Send + 'static,
{
    http::Response::builder()
        .status(code)
        .body(http_body_util::Full::new(body.into()))
        .context("failed to build response")
}

fn print_func_ty(out: &mut String, ty: types::ComponentFunc) {
    out.push_str("func(");
    let mut params = ty.params();
    if let Some((name, ty)) = params.next() {
        out.push_str(name);
        out.push_str(": ");
        print_ty(out, ty);
        for (name, ty) in params {
            out.push_str(", ");
            out.push_str(name);
            out.push_str(": ");
            print_ty(out, ty);
        }
    }
    out.push_str(")");
    let mut results = ty.results();
    if let Some(ty) = results.next() {
        out.push_str(" -> ");
        print_ty(out, ty);
        for ty in results {
            out.push_str(", ");
            print_ty(out, ty);
        }
    }
}

fn print_ty(out: &mut String, ty: Type) {
    #[expect(unused)]
    match ty {
        Type::Bool => out.push_str("bool"),
        Type::S8 => out.push_str("s8"),
        Type::U8 => out.push_str("u8"),
        Type::S16 => out.push_str("s16"),
        Type::U16 => out.push_str("u16"),
        Type::S32 => out.push_str("s32"),
        Type::U32 => out.push_str("u32"),
        Type::S64 => out.push_str("s64"),
        Type::U64 => out.push_str("u64"),
        Type::Float32 => out.push_str("float32"),
        Type::Float64 => out.push_str("float64"),
        Type::Char => out.push_str("char"),
        Type::String => out.push_str("string"),
        Type::List(ty) => {
            out.push_str("list<");
            print_ty(out, ty.ty());
            out.push_str(">");
        }
        Type::Record(ty) => {
            out.push_str("record{");
            let mut fields = ty.fields();
            if let Some(types::Field { name, ty }) = fields.next() {
                out.push_str(name);
                out.push_str(": ");
                print_ty(out, ty);
                for types::Field { name, ty } in fields {
                    out.push_str(", ");
                    out.push_str(name);
                    out.push_str(": ");
                    print_ty(out, ty);
                }
            }
            out.push_str("}");
        }
        Type::Tuple(ty) => {
            out.push_str("tuple<");
            let mut tys = ty.types();
            if let Some(ty) = tys.next() {
                print_ty(out, ty);
                for ty in tys {
                    out.push_str(", ");
                    print_ty(out, ty);
                }
            }
            out.push_str(">");
        }
        Type::Variant(variant) => out.push_str("variant"),
        Type::Enum(_) => out.push_str("enum"),
        Type::Option(option_type) => out.push_str("option"),
        Type::Result(result_type) => out.push_str("result"),
        Type::Flags(flags) => out.push_str("flags"),
        Type::Own(resource_type) => out.push_str("own"),
        Type::Borrow(resource_type) => out.push_str("borrow"),
        Type::Future(future_type) => out.push_str("future"),
        Type::Stream(stream_type) => out.push_str("stream"),
        Type::ErrorContext => out.push_str("error-context"),
    }
}

#[tokio::main]
async fn main() -> wasmtime::Result<()> {
    let args = std::env::args();
    let contracts = args.skip(1).next();
    let contracts = contracts.as_deref().unwrap_or("./contracts");

    let engine = Engine::new(wasmtime::Config::new().async_support(true))?;
    let contracts: HashMap<Box<str>, Contract> = std::fs::read_dir(contracts)?
        .filter_map(|entry| {
            entry
                .map_err(Into::into)
                .and_then(|entry| {
                    let meta = entry.metadata()?;
                    if meta.is_dir() {
                        return Ok(None);
                    }
                    let name = entry.file_name();
                    let Some(name) = name.to_str().and_then(|name| name.strip_suffix(".wasm"))
                    else {
                        return Ok(None);
                    };
                    let wasm = std::fs::read(entry.path())?;
                    let mut wasm = ComponentEncoder::default().module(&wasm)?;
                    let wasm = wasm.encode()?;
                    let contract = Component::new(&engine, &wasm)?;
                    let linker = Linker::new(&engine);
                    let ty = linker.substituted_component_type(&contract)?;
                    let pre = linker.instantiate_pre(&contract)?;
                    Ok(Some((name.into(), Contract { pre, ty })))
                })
                .transpose()
        })
        .collect::<wasmtime::Result<_>>()?;
    let contracts = Arc::new(contracts);
    let srv = hyper::server::conn::http1::Builder::new();
    let lis = TcpListener::bind("[::1]:8080").await?;
    let svc = hyper::service::service_fn({
        move |req: http::Request<Incoming>| {
            let contracts = Arc::clone(&contracts);
            async move {
                let (
                    http::request::Parts {
                        mut headers,
                        method,
                        uri,
                        ..
                    },
                    body,
                ) = req.into_parts();

                if uri.path() != "/" {
                    return build_http_response(
                        http::StatusCode::BAD_REQUEST,
                        format!("URI path `{}` not supported", uri.path()),
                    );
                }
                if let Some(q) = uri.query() {
                    return build_http_response(
                        http::StatusCode::BAD_REQUEST,
                        format!("URI query parameters `{q}` not supported"),
                    );
                }

                let Some(name) = headers.remove("X-Contract") else {
                    return build_http_response(
                        http::StatusCode::BAD_REQUEST,
                        "`X-Contract` header missing",
                    );
                };
                let name = match name.to_str() {
                    Ok(name) => name,
                    Err(err) => {
                        return build_http_response(
                            http::StatusCode::BAD_REQUEST,
                            format!("`X-Contract` header value is not valid UTF-8: {err}"),
                        );
                    }
                };
                let Some(Contract { pre, ty }) = contracts.get(name) else {
                    return build_http_response(
                        http::StatusCode::NOT_FOUND,
                        format!("Contract `{name}` not' found"),
                    );
                };
                let engine = pre.engine();

                match method {
                    http::Method::GET => {
                        let mut out = String::new();
                        for (name, ty) in ty.exports(engine) {
                            match ty {
                                types::ComponentItem::ComponentFunc(ty) => {
                                    out.push_str(name);
                                    out.push_str(": ");
                                    print_func_ty(&mut out, ty);
                                    out.push('\n');
                                }
                                types::ComponentItem::ComponentInstance(ty) => {
                                    let instance = name;
                                    for (name, ty) in ty.exports(engine) {
                                        if let types::ComponentItem::ComponentFunc(ty) = ty {
                                            out.push_str(instance);
                                            out.push_str("#");
                                            out.push_str(name);
                                            out.push_str(": ");
                                            print_func_ty(&mut out, ty);
                                            out.push('\n');
                                        }
                                    }
                                }
                                _ => continue,
                            }
                        }
                        Ok(http::Response::new(http_body_util::Full::new(Bytes::from(
                            out,
                        ))))
                    }
                    http::Method::POST => {
                        let Some(func) = headers.remove("X-Func") else {
                            return build_http_response(
                                http::StatusCode::BAD_REQUEST,
                                "`X-Func` header missing",
                            );
                        };
                        let func = match func.to_str() {
                            Ok(func) => func,
                            Err(err) => {
                                return build_http_response(
                                    http::StatusCode::BAD_REQUEST,
                                    format!("`X-Func` header value is not valid UTF-8: {err}"),
                                );
                            }
                        };

                        let Some(codec) = headers.remove("X-Codec") else {
                            return build_http_response(
                                http::StatusCode::BAD_REQUEST,
                                "`X-Codec` header missing",
                            );
                        };
                        let codec = match codec.to_str() {
                            Ok(codec) => codec,
                            Err(err) => {
                                return build_http_response(
                                    http::StatusCode::BAD_REQUEST,
                                    format!("`X-Codec` header value is not valid UTF-8: {err}"),
                                );
                            }
                        };
                        let url = match Url::parse(codec) {
                            Ok(url) => url,
                            Err(err) => {
                                return build_http_response(
                                    http::StatusCode::BAD_REQUEST,
                                    format!("`X-Codec` header value is not a valid URL: {err}"),
                                );
                            }
                        };
                        let mut codec = if let Ok(path) = url.to_file_path() {
                            match fs::read(&path).await {
                                Ok(codec) => ComponentEncoder::default().module(&codec)?,
                                Err(err) => {
                                    return build_http_response(
                                        http::StatusCode::BAD_REQUEST,
                                        format!(
                                            "Failed to read codec bytes from `{}`: {err}",
                                            path.to_string_lossy()
                                        ),
                                    );
                                }
                            }
                        } else {
                            let res = match reqwest::get(url).await {
                                Ok(codec) => codec,
                                Err(err) => {
                                    return build_http_response(
                                        http::StatusCode::BAD_REQUEST,
                                        format!("Failed to fetch codec from `{codec}`: {err}"),
                                    );
                                }
                            };
                            match res.bytes().await {
                                Ok(codec) => ComponentEncoder::default().module(&codec)?,
                                Err(err) => {
                                    return build_http_response(
                                        http::StatusCode::BAD_REQUEST,
                                        format!(
                                            "Failed to fetch codec bytes from `{codec}`: {err}"
                                        ),
                                    );
                                }
                            }
                        };
                        let codec = codec.encode()?;
                        let codec = Component::new(engine, codec)?;
                        let linker = Linker::new(engine);
                        let mut store = Store::new(engine, ());
                        let codec =
                            bindings::Format::instantiate_async(&mut store, &codec, &linker)
                                .await?;

                        let (func, ty) = if let Some((instance, func)) = func.split_once('#') {
                            let Some(types::ComponentItem::ComponentInstance(ty)) =
                                ty.get_export(engine, instance)
                            else {
                                return build_http_response(
                                    http::StatusCode::NOT_FOUND,
                                    format!("Instance `{instance}` not' found"),
                                );
                            };
                            let Some(types::ComponentItem::ComponentFunc(ty)) =
                                ty.get_export(engine, func)
                            else {
                                return build_http_response(
                                    http::StatusCode::NOT_FOUND,
                                    format!("Function `{func}` not found in instance `{instance}`"),
                                );
                            };

                            let contract = pre.instantiate_async(&mut store).await?;
                            let (_, instance) = contract
                                .get_export(&mut store, None, instance)
                                .expect("instance export not found");
                            let (_, func) = contract
                                .get_export(&mut store, Some(&instance), func)
                                .expect("function export not found");
                            let func = contract
                                .get_func(&mut store, func)
                                .expect("function not found");
                            (func, ty)
                        } else {
                            let Some(types::ComponentItem::ComponentFunc(ty)) =
                                ty.get_export(engine, func)
                            else {
                                return build_http_response(
                                    http::StatusCode::NOT_FOUND,
                                    format!("Function `{func}` not' found"),
                                );
                            };
                            let contract = pre.instantiate_async(&mut store).await?;
                            let func = contract
                                .get_func(&mut store, func)
                                .expect("export not found");
                            (func, ty)
                        };
                        let params = deserialize_params(&mut store, &codec, &ty, body).await?;
                        let mut results = vec![Val::Bool(false); ty.results().len()];
                        func.call_async(&mut store, &params, &mut results)
                            .await
                            .context("failed to call function")?;
                        Ok(http::Response::new(http_body_util::Full::new(Bytes::from(
                            format!("{results:?}"),
                        ))))
                    }
                    method => build_http_response(
                        http::StatusCode::METHOD_NOT_ALLOWED,
                        format!("Method `{method}` not supported"),
                    ),
                }
            }
        }
    });
    loop {
        let (stream, _) = lis.accept().await?;
        let conn = srv.serve_connection(TokioIo::new(stream), svc.clone());
        tokio::spawn(async {
            if let Err(err) = conn.await {
                eprintln!("failed to serve connection: {err:?}");
            }
        });
    }
}
