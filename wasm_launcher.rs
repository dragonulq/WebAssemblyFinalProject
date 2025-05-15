use anyhow::{anyhow, Context, Result};
use std::{env, fs, process};
use wasmtime::{Linker, Module, Store, Config, ExternType, Memory, Table, Ref, Global};
use wasmtime_wasi::WasiP1Ctx;
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::preview1::add_to_linker_sync;
use wasmtime_wasi::I32Exit;
mod helpers;
use helpers::dependency_order;
use helpers::collect_env_imports;
use helpers::zero_for;
use helpers::remove_duplicates;

fn main() -> Result<()> {

    let mut args = env::args();
    let prog = args.next().expect("argv[0] missing");
    let wasm_path = match args
        .next()
        .map(|p| fs::canonicalize(p).context(".wasm binary cannot be found!"))
        .transpose()? {
        Some(path) => path,

        None => {
        eprintln!("usage: {prog} <file.wasm>");
        process::exit(1);
        }
    };

    let argv: Vec<String> = env::args().skip(1).collect();
    let config = Config::new();
    let engine = wasmtime::Engine::new(&config)?;


    let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
    add_to_linker_sync(&mut linker, |t| t)?;

    let mut wasi_ctx_builder = WasiCtxBuilder::new();
    let wasi_ctx = (&mut wasi_ctx_builder)
        .inherit_stdio()
        .args(&argv)
        .build_p1();


    let mut store = Store::new(&engine, wasi_ctx);
    let modules_to_be_instantiated = dependency_order(&engine, &wasm_path.as_path())?;

    let modules_to_be_instantiated_len = modules_to_be_instantiated.len();
    let mut instance = None;
    let modules_to_be_instantiated = remove_duplicates(modules_to_be_instantiated);
    for (i, (module_name, module_path)) in modules_to_be_instantiated.iter().enumerate() {

        let module = Module::from_file(&engine, &module_path)
            .with_context(|| format!("Could not compile {}", module_path.display()))?;

        for (name, ty) in collect_env_imports(&module) {
            match ty {

                ExternType::Memory(mt) => {
                    let mem = Memory::new(&mut store, mt)?;
                    linker.define(&store, "env", &name, mem)?;
                }

                ExternType::Table(tt) => {
                    let init = Ref::Func(None);
                    let table = Table::new(&mut store, tt, init)?;
                    linker.define(&store, "env", &name, table)?;
                }

                ExternType::Global(gt) => {
                    if let Some(extern_instance) = linker.get(&mut store, "env", &name) {

                        match extern_instance.into_global() {
                            None => anyhow::bail!("type mismatch for env::{name}: module expects GlobalType but host provided something else!"),
                            _ => {}
                        }

                    } else {
                        let glob = &gt.content();
                        let val = zero_for(&glob);
                        let new_glob = Global::new(&mut store, gt, val)?;
                        linker.define(&store, "env", &name, new_glob)?;
                    }
                }

                ExternType::Func(_ft) => {
                    if let Some(extern_instance) = linker.get(&mut store, "env", &name) {

                        match extern_instance.into_func() {
                            None => anyhow::bail!("type mismatch for env::{name}: module expects FuncType but host provided something else!"),
                            _ => {}
                        }

                    } else {
                        anyhow::bail!("Providing implementations for random functions not ready yet!");
                    }
                }

            }
        }

        instance = Some(linker.instantiate(&mut store, &module)?);
        if i <= modules_to_be_instantiated_len - 1 {
            let unwrapped_instance = match instance {
                Some(i) => i,
                None => anyhow::bail!("Ended up with None value for instance"),
            };
            linker.instance(&mut store, &module_name, unwrapped_instance)?;
        }
    }

    let instance = match instance {
        Some(i) => i,
        None => anyhow::bail!("Ended up with None value for instance"),
    };

    let _start = instance
        .get_func(&mut store, "_start")
        .ok_or_else(|| anyhow!("export `_start` not found"))?;

    match _start.call(&mut store, &[], &mut []) {
        Ok(()) => {
            println!("Module exited normally");
        }
        Err(e) => {

            if let Some(exit) = e.downcast_ref::<I32Exit>() {
                let code = exit.0;
                println!("guest exited with status {code}");
                std::process::exit(code as i32);
            } else {
                return Err(e).context("failed to run guest")?;
            }
        }
    }

    Ok(())
}
