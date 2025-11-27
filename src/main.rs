mod bindings {
    wasmtime::component::bindgen!({
        world: "format",
        exports: {
            default: async,
        }
    });
}

use core::mem;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context as _, anyhow};
use bytes::{Buf, Bytes};
use http_body_util::BodyExt as _;
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use tokio::fs;
use tokio::net::TcpListener;
use url::Url;
use wasmtime::component::{Component, InstancePre, Linker, ResourceAny, Type, Val, types};
use wasmtime::{Engine, Store};
use wit_component::ComponentEncoder;

use bindings::exports::rvolosatovs::serde::deserializer::Guest;

pub struct Error;

async fn handle_deserialized_value<T>(
    store: &mut Store<()>,
    instance: &Guest,
    res: Result<T, ResourceAny>,
    v: &mut Val,
    mk_val: impl FnOnce(T) -> Val,
) -> wasmtime::Result<()> {
    match res {
        Ok(de) => {
            *v = mk_val(de);
            Ok(())
        }
        Err(err) => {
            let s = instance.error().call_to_string(store, err).await?;
            Err(anyhow!(s).context("failed to deserialize string"))
        }
    }
}

async fn deserialize(
    store: &mut Store<()>,
    de: ResourceAny,
    instance: &Guest,
    ty: Type,
    v: &mut Val,
) -> wasmtime::Result<()> {
    #[expect(unused, reason = "incomplete")]
    match ty {
        Type::Bool => {
            let res = instance
                .deserializer()
                .call_deserialize_bool(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::Bool).await
        }
        Type::S8 => {
            let res = instance
                .deserializer()
                .call_deserialize_s8(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::S8).await
        }
        Type::U8 => {
            let res = instance
                .deserializer()
                .call_deserialize_u8(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::U8).await
        }
        Type::S16 => {
            let res = instance
                .deserializer()
                .call_deserialize_s16(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::S16).await
        }
        Type::U16 => {
            let res = instance
                .deserializer()
                .call_deserialize_u16(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::U16).await
        }
        Type::S32 => {
            let res = instance
                .deserializer()
                .call_deserialize_s32(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::S32).await
        }
        Type::U32 => {
            let res = instance
                .deserializer()
                .call_deserialize_u32(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::U32).await
        }
        Type::S64 => {
            let res = instance
                .deserializer()
                .call_deserialize_s64(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::S64).await
        }
        Type::U64 => {
            let res = instance
                .deserializer()
                .call_deserialize_u64(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::U64).await
        }
        Type::Float32 => {
            let res = instance
                .deserializer()
                .call_deserialize_f32(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::Float32).await
        }
        Type::Float64 => {
            let res = instance
                .deserializer()
                .call_deserialize_f64(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::Float64).await
        }
        Type::Char => {
            let res = instance
                .deserializer()
                .call_deserialize_char(&mut *store, de)
                .await?;
            handle_deserialized_value(store, instance, res, v, Val::Char).await
        }
        Type::String => match instance
            .deserializer()
            .call_deserialize_string(&mut *store, de)
            .await?
        {
            Ok(s) => {
                *v = Val::String(s);
                Ok(())
            }
            Err(err) => {
                let s = instance.error().call_to_string(store, err).await?;
                Err(anyhow!(s).context("failed to deserialize string"))
            }
        },
        Type::List(ty) => todo!(),
        Type::Record(ty) => {
            let mut names = ty
                .fields()
                .map(|types::Field { name, .. }| name.into())
                .collect::<Vec<_>>();
            let tys = ty
                .fields()
                .map(|types::Field { ty, .. }| ty)
                .collect::<Vec<_>>();
            match instance
                .deserializer()
                .call_deserialize_record(&mut *store, de, todo!())
                .await?
            {
                Ok((mut idx, mut de, mut iter)) => {
                    let num_fields = ty.fields().len();
                    let mut vs = Vec::with_capacity(num_fields);
                    let mut fv = Val::Bool(false);
                    Box::pin(deserialize(
                        &mut *store,
                        de,
                        instance,
                        tys[idx as usize].clone(),
                        &mut fv,
                    ))
                    .await
                    .with_context(|| {
                        format!("failed to deserialize record field with index `{idx}`")
                    })?;
                    vs.push((idx, fv));
                    for _ in 1..num_fields {
                        let next = instance
                            .record_deserializer()
                            .call_next(&mut *store, iter)
                            .await
                            .context("failed to call `next`")?;
                        idx = next.0;
                        de = next.1;
                        iter = next.2;
                        let mut fv = Val::Bool(false);
                        Box::pin(deserialize(
                            &mut *store,
                            de,
                            instance,
                            tys[idx as usize].clone(),
                            &mut fv,
                        ))
                        .await
                        .with_context(|| {
                            format!("failed to deserialize record field with index `{idx}`")
                        })?;
                        vs.push((idx, fv));
                    }
                    vs.sort_unstable_by(|(l, ..), (r, ..)| l.cmp(r));
                    let vs = vs
                        .into_iter()
                        .map(|(idx, v)| (mem::take(&mut names[idx as usize]), v))
                        .collect();
                    *v = Val::Record(vs);
                    Ok(())
                }
                Err(err) => {
                    let err = instance.error().call_to_string(store, err).await?;
                    Err(anyhow!(err))
                }
            }
        }
        Type::Tuple(ty) => {
            let mut tys = ty.types();
            let num_elements = tys.len();
            match instance
                .deserializer()
                .call_deserialize_tuple(&mut *store, de, todo!())
                .await?
            {
                Ok((mut de, mut iter)) => {
                    let mut vs = Vec::with_capacity(num_elements);
                    let mut ev = Val::Bool(false);
                    let ty = tys
                        .next()
                        .context("failed to get first tuple element type")?;
                    Box::pin(deserialize(&mut *store, de, instance, ty, &mut ev))
                        .await
                        .context("failed to deserialize first tuple element")?;
                    vs.push(ev);
                    for ty in tys {
                        let next = instance
                            .tuple_deserializer()
                            .call_next(&mut *store, iter)
                            .await
                            .context("failed to call `next`")?;
                        de = next.0;
                        iter = next.1;
                        let mut ev = Val::Bool(false);
                        Box::pin(deserialize(&mut *store, de, instance, ty, &mut ev))
                            .await
                            .context("failed to deserialize tuple element")?;
                        vs.push(ev);
                    }
                    *v = Val::Tuple(vs);
                    Ok(())
                }
                Err(err) => {
                    let err = instance.error().call_to_string(store, err).await?;
                    Err(anyhow!(err))
                }
            }
        }
        Type::Variant(ty) => todo!(),
        Type::Enum(ty) => todo!(),
        Type::Option(ty) => todo!(),
        Type::Result(ty) => todo!(),
        Type::Flags(ty) => todo!(),
        Type::Own(ty) => todo!(),
        Type::Borrow(ty) => todo!(),
        Type::Future(ty) => todo!(),
        Type::Stream(ty) => todo!(),
        Type::ErrorContext => todo!(),
    }
}

async fn deserialize_params(
    store: &mut Store<()>,
    instance: &Guest,
    ty: &types::ComponentFunc,
    body: hyper::body::Incoming,
) -> wasmtime::Result<Vec<Val>> {
    let mut tys = ty.params();
    let num_params = tys.len();
    if num_params == 0 {
        // TODO: Ensure that body is empty
        return Ok(Vec::default());
    }
    let body = body.collect().await?;
    let de = instance
        .deserializer()
        .call_from_list(&mut *store, &body.to_bytes())
        .await?;

    // TODO: impl
    match instance
        .deserializer()
        .call_deserialize_tuple(&mut *store, de, todo!())
        .await?
    {
        Ok((mut de, mut iter)) => {
            let mut vs = Vec::with_capacity(num_params);
            let mut pv = Val::Bool(false);
            let (name, ty) = tys.next().context("failed to get first parameter type")?;
            Box::pin(deserialize(&mut *store, de, instance, ty, &mut pv))
                .await
                .with_context(|| format!("failed to deserialize param `{name}`"))?;
            vs.push(pv);
            for (name, ty) in tys {
                let next = instance
                    .tuple_deserializer()
                    .call_next(&mut *store, iter)
                    .await
                    .context("failed to call `next`")?;
                de = next.0;
                iter = next.1;
                let mut pv = Val::Bool(false);
                Box::pin(deserialize(&mut *store, de, instance, ty, &mut pv))
                    .await
                    .with_context(|| format!("failed to deserialize param `{name}`"))?;
                vs.push(pv);
            }
            Ok(vs)
        }
        Err(err) => {
            let err = instance.error().call_to_string(store, err).await?;
            Err(anyhow!(err))
        }
    }
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
                        let codec = codec.rvolosatovs_serde_deserializer();

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
                        let params = deserialize_params(&mut store, codec, &ty, body).await?;
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
