use anyhow::{anyhow, Context, Result};
use std::sync::{Mutex, OnceLock};
use std::{env, fs, process};
use wasmtime::{AsContextMut, Engine, Linker, Module, Store, Instance};
use wasmtime_wasi::preview1::add_to_linker_sync;
use wasmtime_wasi::I32Exit;
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::WasiP1Ctx;
mod helpers;
mod dl_functions;

use dl_functions::{make_wasm_dlopen, make_wasm_dlcall};
use helpers::{dependency_order, remove_duplicates, get_instance_memory_copy};

struct GlobalWasmCtx {
    engine: Engine,
    linker: Mutex<Linker<WasiP1Ctx>>,
}

struct Instances {
    instances: Mutex<Vec<Instance>>,
}

impl Instances {
    fn new() -> Self {
        let instances = Vec::new();
        Self {instances: Mutex::new(instances)}
    }
}

impl GlobalWasmCtx {
    fn new() -> Self {
        let engine = Engine::default();
        let linker = Linker::new(&engine);

        Self {
            engine,
            linker: Mutex::new(linker),
        }
    }
}

fn get_global_objects() -> &'static GlobalWasmCtx {
    static GLOBAL_OBJECTS: OnceLock<GlobalWasmCtx> = OnceLock::new();
    GLOBAL_OBJECTS.get_or_init(|| GlobalWasmCtx::new())
}

fn get_instances() -> &'static Instances {
    static INSTANCES: OnceLock<Instances> = OnceLock::new();
    INSTANCES.get_or_init(|| Instances::new())
}

//TODO start refactoring logic out of main()
fn main() -> Result<()> {
    let mut args = env::args();
    let prog = args.next().expect("argv[0] missing");
    let wasm_path = match args
        .next()
        .map(|p| fs::canonicalize(p).context(".wasm binary cannot be found!"))
        .transpose()?
    {
        Some(path) => path,

        None => {
            eprintln!("usage: {prog} <file.wasm>");
            process::exit(1);
        }
    };

    let argv: Vec<String> = env::args().skip(1).collect();
    let global_objects = get_global_objects();
    let engine = &global_objects.engine;

    let mut wasi_ctx_builder = WasiCtxBuilder::new();
    let wasi_ctx = (&mut wasi_ctx_builder)
        .inherit_stdio()
        .args(&argv)
        .build_p1();

    let mut store = Store::new(&engine, wasi_ctx);
    let instance:Instance = {
        let linker_guard = &mut global_objects.linker.lock().unwrap();
        let linker: &mut Linker<WasiP1Ctx> = &mut *linker_guard;
        add_to_linker_sync(linker, |t| t)?;

        let modules_to_be_instantiated = dependency_order(&engine, &wasm_path.as_path())?;
        let dlopen_func = make_wasm_dlopen(&mut store);
        let dlcall_func = make_wasm_dlcall(&mut store);
                
        linker.define(store.as_context_mut(), "host", "wasm_dlopen", dlopen_func)?;
        linker.define(store.as_context_mut(), "host", "wasm_dlcall", dlcall_func)?;

        let modules_to_be_instantiated_len = modules_to_be_instantiated.len();
        let mut instance = None;
        let modules_to_be_instantiated = remove_duplicates(modules_to_be_instantiated);
        for (i, (module_name, module_path)) in modules_to_be_instantiated.iter().enumerate() {
            let module = Module::from_file(&engine, &module_path)
                .with_context(|| format!("Could not compile {}", module_path.display()))?;

            instance = Some(linker.instantiate(&mut store, &module)?);
            if i <= modules_to_be_instantiated_len - 1 {
                let unwrapped_instance = match instance {
                    Some(i) => i,
                    None => anyhow::bail!("Ended up with None value for instance"),
                };
                linker.instance(&mut store, &module_name, unwrapped_instance)?;
            }
        }

        match instance {
            Some(i) => i,
            None => anyhow::bail!("Ended up with None value for instance"),
        }
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
