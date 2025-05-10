use anyhow::{anyhow, Context, Result};
use std::{env, fs, process};
use wasmtime::{Engine, Func, Instance, Module, Store, Val, Caller, FuncType};
fn prepare_wasm_args(
    func_ty: &FuncType,
    raw_args: impl Iterator<Item = String>,
) -> Result<Vec<Val>> {

    let param_tys = func_ty.params();

    // Convert each argument based on expected type
    let mut converted = Vec::new();
    for (i, (arg, ty)) in raw_args.zip(param_tys).enumerate() {
        converted.push(match ty {
            wasmtime::ValType::I32 => Val::I32(arg.parse().context(format!(
                "Argument {} (`{}`) must be an i32",
                i + 1, arg
            ))?),
            wasmtime::ValType::I64 => Val::I64(arg.parse().context(format!(
                "Argument {} (`{}`) must be an i64",
                i + 1, arg
            ))?),
            wasmtime::ValType::F32 => Val::F32(arg.parse().context(format!(
                "Argument {} (`{}`) must be an f32",
                i + 1, arg
            ))?),
            wasmtime::ValType::F64 => Val::F64(arg.parse().context(format!(
                "Argument {} (`{}`) must be an f64",
                i + 1, arg
            ))?),
            _ =>  return Err(anyhow!("Passed argument (`{}`) is not supported", arg)),
        });
    }

    Ok(converted)
}

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


    println!("Initializing…");
    println!("Arg 1 (wasm): {}", wasm_path.display());

    let engine = Engine::default();
    let mut store = Store::new(&engine, ());


    let wasm_bytes = fs::read(&wasm_path)
        .with_context(|| format!("failed to read {}", wasm_path.display()))?;

    println!("Compiling module…");
    let module = Module::from_binary(&engine, &wasm_bytes)
        .context("failed to compile module")?;


    println!("Creating callback…");

    let hello = Func::wrap(&mut store, |_caller: Caller<'_, ()>, arg1: i32, arg2: i32| {
        println!("Calling back…");
        println!("> Hello World From WASM module!");
        println!("Sum of {:?} and {:?}", arg1, arg2);
    });

    println!("Instantiating module…");
    let instance = Instance::new(&mut store, &module, &[hello.into()])
        .context("failed to instantiate")?;


    println!("Extracting export…");
    let _start = instance
        .get_func(&mut store, "_start")
        .ok_or_else(|| anyhow!("export `_start` not found"))?;

    let _start_ty = _start.ty(&mut store);

    let wasm_args = prepare_wasm_args(&_start_ty, args)?;

    println!("Calling export…");
    _start.call(&mut store, &wasm_args, &mut [])
        .context("failed to call `_start`")?;

    println!("All finished!");
    Ok(())
}
