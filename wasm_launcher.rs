use anyhow::{anyhow, Context, Result};
use std::{env, fs, process};
use wasmtime::{Linker, Module, Store, Config};
use wasmtime_wasi::WasiP1Ctx;
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::preview1::add_to_linker_sync;
use wasmtime_wasi::I32Exit;
mod helpers;
use helpers::dependency_order;

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
    for (i, (module_name, module_path)) in modules_to_be_instantiated.iter().enumerate() {
        if i == modules_to_be_instantiated_len - 1 {
            break;
        }
        let module = Module::from_file(&engine, &module_path)
            .with_context(|| format!("Could not compile {}", module_path.display()))?;
        let instance = linker.instantiate(&mut store, &module)?;

        linker.instance(&mut store, &module_name, instance)?;

    }

    let module = Module::from_file(&engine, &wasm_path)?;
    let instance = linker.instantiate(&mut store, &module)?;

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
