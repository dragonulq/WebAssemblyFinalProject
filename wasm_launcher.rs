use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::sync::{Arc, Mutex, RwLock};
use std::thread_local;
use std::{env, fs, process};
use wasmtime::{AsContextMut, Caller, Config, Engine, Func, Linker, Module, Store};
use wasmtime_wasi::preview1::add_to_linker_sync;
use wasmtime_wasi::I32Exit;
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::WasiP1Ctx;

mod helpers;
// mod dl_functions;

use helpers::dependency_order;
use helpers::remove_duplicates;

struct GlobalWasmCtx {
    engine: Engine,
    store: Store<WasiP1Ctx>,
    linker: Linker<WasiP1Ctx>,
}

impl GlobalWasmCtx {
    fn new() -> Self {
        
        let engine = Engine::default();

       
        let mut wasi_ctx_builder = WasiCtxBuilder::new();
        
        let wasi_ctx = (&mut wasi_ctx_builder)
            .inherit_stdio()
            .build_p1();
      
        let mut store = Store::new(&engine, wasi_ctx);
        let mut linker = Linker::new(&engine);
        
        Self {
            engine,
            store,
            linker
        }
    }
}

// thread_local! {
//     static GLOBAL_OBJECTS: GlobalWasmCtx = GlobalWasmCtx::new();
// }
thread_local! {
    static GLOBAL_OBJECTS: RefCell<GlobalWasmCtx> = RefCell::new(GlobalWasmCtx::new());
}

// static GLOBAL_OBJECTS: Lazy<Arc<GlobalWasmCtx>> = Lazy::new(|| {
//    
//     let config = Config::new();
//     let engine = wasmtime::Engine::new(&config).unwrap();
//
//     
//     let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
//     add_to_linker_sync(&mut linker, |t| t).unwrap(); 
//
//     Arc::new(GlobalWasmCtx::new())
// });

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
    let config = Config::new();
    let engine = wasmtime::Engine::new(&config)?;

    let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);

    add_to_linker_sync(&mut linker, |t| t)?;

    let mut wasi_ctx_builder = WasiCtxBuilder::new();
    let wasi_ctx = (&mut wasi_ctx_builder)
        .inherit_stdio()
        .args(&argv)
        .build_p1();



    // Create store
    let mut store = Store::new(&engine, wasi_ctx);
    // let mut store = &mut GLOBAL_OBJECTS.clone().store;
    let modules_to_be_instantiated = dependency_order(&engine, &wasm_path.as_path())?;
    let dlopen_func = Func::wrap(&mut store, |caller: Caller<'_, WasiP1Ctx>, ptr: i32, flags: i32| -> i32 {
       
        //state.linker.borrow_mut().engine();
        GLOBAL_OBJECTS.with(|ctx| {

            // let linker = &ctx.borrow().linker;
            // let linker = &mut ctx.borrow_mut().linker;
            // let store = &mut ctx.borrow_mut().store;
            // // let engine =  &ctx.borrow_mut().engine;
            // let module = Module::from_file(engine,"test-dlopen.wasm").unwrap();
            let module = Module::from_file(&ctx.borrow_mut().engine,"test-dlopen.wasm").unwrap();
            
            // let instance = linker.instantiate(store, &module).unwrap();
            let instance = (&mut ctx.borrow_mut().linker).instantiate(&mut ctx.borrow_mut().store, &module).unwrap();
            //
        });
        // linker.engine();
        println!("host_dlopen called with ptr={}, flags={}", ptr, flags);
        70
    });
    linker.define(&store,"host", "host_dlopen", dlopen_func)?;

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
