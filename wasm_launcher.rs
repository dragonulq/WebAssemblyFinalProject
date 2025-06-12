use std::path::PathBuf;
use wasmtime::Config;
use anyhow::{anyhow, Context, Result};
use std::sync::{Mutex, OnceLock};
use std::{env, fs, process};
use wasmtime::{AsContextMut, Engine, Linker, Module, Store, Instance};

use wasmtime_wasi::I32Exit;
use wasmtime_wasi::{WasiCtxBuilder};

use wasmtime_wasi::preview1::WasiP1Ctx;
use cap_std::fs::Dir;
use wasmtime_wasi::DirPerms;
use wasmtime_wasi::FilePerms;
use wasmtime_wasi::preview1::add_to_linker_sync;


mod helpers;
mod dl_functions;


use dl_functions::{make_wasm_dlopen, make_wasm_dlcall, make_wasm_dlopen2, make_write_to_host_buffer};
use helpers::{dependency_order, remove_duplicates, get_name_from_memory};
use crate::dl_functions::make_read_from_host_buffer;

struct GlobalWasmCtx {
    engine: Engine,
    linker: Mutex<Linker<WasiP1Ctx>>,
}

struct Instances {
    instances: Mutex<Vec<Instance>>,
}
const BUFFER_SIZE: usize = 1000000000; // 10 ^ 9 bytes

static mut DLCALL_BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

impl Instances {
    fn new() -> Self {
        let instances = Vec::new();
        Self {instances: Mutex::new(instances)}
    }
}

impl GlobalWasmCtx {
    fn new() -> Self {
        let mut config = Config::new();
        config.max_wasm_stack(50 * 1024 * 1024);
        let engine = match Engine::new(&config) {
            Ok(e) => e,
            Err(_) => panic!("Failed to create engine with a Config!"),
        };
        // let engine = Engine::default();
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
//TODO replace debug with optimized build CPython.wasm
fn main() -> Result<()> {
    // unsafe {
    //     let first_50 = &DLCALL_BUFFER[..52];
    //     println!("First 50 bytes: {:?}", first_50);
    // }
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

    let cwd: PathBuf = env::current_dir()?;
    let host_path = cwd.join("cpython/cross-build/wasm32-wasip1");

    wasi_ctx_builder.preopened_dir(host_path, "/", DirPerms::all(), FilePerms::all())?;
    wasi_ctx_builder.env("PYTHONPATH", "/build/lib.wasi-wasm32-3.15:/Lib");
    wasi_ctx_builder.env("PYTHONHOME", "/");


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
        let wasm_dlopen2 = make_wasm_dlopen2(&mut store);
        let write_to_host_buffer = make_write_to_host_buffer(&mut store);
        let read_from_host_buffer = make_read_from_host_buffer(&mut store);
                
        linker.define(store.as_context_mut(), "host", "wasm_dlopen", dlopen_func)?;
        linker.define(store.as_context_mut(), "host", "wasm_dlcall", dlcall_func)?;
        linker.define(store.as_context_mut(), "host", "wasm_dlopen2", wasm_dlopen2)?;
        linker.define(store.as_context_mut(), "host", "write_to_host_buffer", write_to_host_buffer)?;
        linker.define(store.as_context_mut(), "host", "read_from_host_buffer", read_from_host_buffer)?;

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
    // unsafe {
    //     let first_50 = &DLCALL_BUFFER[..52];
    //     println!("First 50 bytes: {:?}", first_50);
    // }
    Ok(())
}
