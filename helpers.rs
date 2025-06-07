use anyhow::{Context, Result};
use std::{
    path::{Path, PathBuf},
};
use std::collections::HashSet;
use wasmtime::{Engine, Module, Extern, Caller, AsContext, AsContextMut, Memory};
use wasmtime_wasi::preview1::WasiP1Ctx;


pub fn dependency_order(engine: &Engine, root: &Path) -> Result<Vec<(String, PathBuf)>> {
    dfs(engine, root)
}
fn dfs(engine: &Engine, path: &Path) -> Result<Vec<(String, PathBuf)>> {
/*TODO rethink the algorithm or probably just which duplicates to remove
   either the second ones in which the implementation is correct or the first ones
    in which case we traverse the list backwards*/
    let module = Module::from_file(engine, path) //TODO persisting to OS could come in handy here
        .with_context(|| format!("Could not compile {}", path.display()))?;


    let mut list = Vec::<(String, PathBuf)>::new();
    for imp in module.imports() {
        if imp.module() == "wasi_snapshot_preview1" || imp.module() == "env"  || imp.module() == "host" {
            continue;
        }
        let dep = path.with_file_name(format!("{}.wasm", imp.module()));
        list.extend(dfs(engine, &dep)?);
    }

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    list.push((name, path.to_path_buf()));
    Ok(list)
}
//TODO does this iter preserve order, probably yes, but check
pub fn remove_duplicates(vec: Vec<(String, PathBuf)>) -> Vec<(String, PathBuf)> {
    let mut seen_strings = HashSet::new();
    vec.into_iter()
        .filter(|(s, _)| {
            if seen_strings.contains(s) {
                false
            } else {
                seen_strings.insert(s.clone());
                true
            }
        })
        .collect()
}

pub fn get_name_from_memory(caller: &mut Caller<'_, WasiP1Ctx>, ptr: i32, name_len: i32) -> String {

    const NAME_MAX_LENGTH: i32 = 4096;

    let mut backing_array = [0u8; NAME_MAX_LENGTH as usize];
    let buffer: &mut [u8] = &mut backing_array[0..name_len as usize];

    let memory =  match caller.get_export("memory") {
        Some(Extern::Memory(memory)) => memory.clone(),   // clone the reference to it
        _ => panic!("missing memory export!")
    };

    match memory.read(caller.as_context(), ptr as usize, buffer) {
        Ok(()) => {},
        _ => panic!("Something went wrong while reading guest memory to get library name!")
    }

    match std::str::from_utf8(&buffer[..name_len as usize]) {
        Ok(s) => s.to_string(),
        Err(_) => panic!("Could not convert buffer to string!"),
    }

}

pub fn read_bytes_from_module(caller: &mut Caller<'_, WasiP1Ctx>, buffer: &mut [u8], arg_ptr: i32) {
    let memory =  match caller.get_export("memory") {
        Some(Extern::Memory(memory)) => memory.clone(),   // clone the reference to it
        _ => panic!("missing memory export!")
    };

    match memory.read(caller.as_context(), arg_ptr as usize, buffer) {
        Ok(()) => {},
        _ => panic!("Something went wrong while reading guest memory to get library name!")
    }

}
//TODO reallocate in case Memory is full
pub fn write_bytes_to_module(caller: &mut Caller<'_, WasiP1Ctx>, memory: Memory, ptr: i32, buffer_to_copy: &mut [u8]) {
    let _ = memory.write(caller.as_context_mut(), ptr as usize, buffer_to_copy);
}