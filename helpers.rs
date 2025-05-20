use anyhow::{Context, Result};
use std::{
    path::{Path, PathBuf},
};
use std::collections::HashSet;
use wasmtime::{Engine, Module, Extern, Caller, Memory};
use wasmtime_wasi::WasiP1Ctx;

pub fn dependency_order(engine: &Engine, root: &Path) -> Result<Vec<(String, PathBuf)>> {
    dfs(engine, root)
}
fn dfs(engine: &Engine, path: &Path) -> Result<Vec<(String, PathBuf)>> {

    let module = Module::from_file(engine, path)
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

pub fn get_instance_memory_copy(caller: &mut Caller<'_, WasiP1Ctx>) -> Memory {
    match caller.get_export("memory") {
        Some(Extern::Memory(memory)) => memory.clone(),
        _ => panic!("missing memory export!")
    }
}

