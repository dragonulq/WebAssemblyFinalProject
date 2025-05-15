use anyhow::{Context, Result};
use std::{
    path::{Path, PathBuf},
};
use std::collections::HashSet;
use wasmtime::{Engine, Module, ExternType, ValType, Val};

pub fn dependency_order(engine: &Engine, root: &Path) -> Result<Vec<(String, PathBuf)>> {
    dfs(engine, root)
}
fn dfs(engine: &Engine, path: &Path) -> Result<Vec<(String, PathBuf)>> {

    let module = Module::from_file(engine, path)
        .with_context(|| format!("Could not compile {}", path.display()))?;


    let mut list = Vec::<(String, PathBuf)>::new();
    for imp in module.imports() {
        if imp.module() == "wasi_snapshot_preview1" || imp.module() == "env" {
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

pub fn collect_env_imports(module: &Module) -> Vec<(String, ExternType)> {

    module
        .imports()
        .filter(|imp| imp.module() == "env")
        .map(|imp| (imp.name().to_string(), imp.ty().clone()))
        .collect()
}

pub fn zero_for(ty: &ValType) -> Val {
    match ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0),
        ValType::F64 => Val::F64(0),
        other => panic!("unsupported global type {other:?}"),
    }
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

