use anyhow::{anyhow, Context, Result};
use std::{env, fs, process};
use wasmtime::{Engine, Func, Linker, Instance, Module, Store, Caller, Config};
use wasmtime_wasi::WasiP1Ctx;
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::preview1::add_to_linker_sync;
use wasmtime_wasi::I32Exit;

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
    let mut config = Config::new();
    let engine = wasmtime::Engine::new(&config)?;


    let wasm_bytes = fs::read(&wasm_path)
        .with_context(|| format!("failed to read {}", wasm_path.display()))?;

    println!("Compiling module…");
    let module = Module::from_binary(&engine, &wasm_bytes)
        .context("failed to compile module")?;


    let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
    add_to_linker_sync(&mut linker, |t| t)?;
    let pre = linker.instantiate_pre(&module)?;

    let mut wasi_ctx_builder = WasiCtxBuilder::new();
    let wasi_ctx = (&mut wasi_ctx_builder)
        .inherit_stdio()
        .args(&argv)
        .build_p1();


    let mut store = Store::new(&engine, wasi_ctx);
    let instance = linker.instantiate(&mut store, &module)?;


    println!("Extracting export…");
    let _start = instance
        .get_func(&mut store, "_start")
        .ok_or_else(|| anyhow!("export `_start` not found"))?;


    println!("Calling export…");

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


    println!("All finished!");
    Ok(())
}
