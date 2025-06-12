use std::path::PathBuf;
use std::env;
use wasmtime::{AsContextMut, Caller, Func, Linker, Val, Module, Instance, Extern, AsContext};
use wasmtime_wasi;
use wasmtime_wasi::preview1::WasiP1Ctx;

use crate::{get_global_objects, get_instances, get_name_from_memory, DLCALL_BUFFER};
use crate::helpers::{read_bytes_from_module, write_bytes_to_module};

pub fn make_write_to_host_buffer(mut store: impl AsContextMut<Data = WasiP1Ctx>) -> Func {
    return Func::wrap(
        store.as_context_mut(),
        |mut caller: Caller<'_, WasiP1Ctx>, data_ptr: i32, data_len: i32| -> i32  {
            let mut success = 0;
            unsafe {
                let guest_memory = caller.get_export("memory")
                    .unwrap_or_else(|| panic!("Guest doesn't export name 'memory'"));
                let guest_memory = guest_memory.into_memory()
                    .unwrap_or_else(|| panic!("Guest doesn't export memory object with name 'memory'"));
                
                match guest_memory.read(caller.as_context(), data_ptr as usize, &mut DLCALL_BUFFER[0..(data_len as usize)]) {
                    Ok(res) => 0,
                    _ => {success = 1;1}
                };
            }
            success
        }
    );
}

pub fn make_read_from_host_buffer(mut store: impl AsContextMut<Data = WasiP1Ctx>) -> Func {
    return Func::wrap(
        store.as_context_mut(),
        |mut caller: Caller<'_, WasiP1Ctx>, target_ptr: i32, data_len: i32| -> i32  {
            let mut success = 0;
            unsafe {
                let guest_memory = caller.get_export("memory")
                    .unwrap_or_else(|| panic!("Guest doesn't export name 'memory'"))
                    .into_memory()
                    .unwrap_or_else(|| panic!("Guest doesn't export memory object with name 'memory'"));
                
                match guest_memory
                    .write(caller.as_context_mut(), target_ptr as usize, &mut DLCALL_BUFFER[0..(data_len as usize)]) {
                    Ok(_) => 0,
                    _ => {success = 1;1}
                };
                
            }
            success
        }
    );
}


pub fn make_wasm_dlopen(mut store: impl AsContextMut<Data = WasiP1Ctx>) -> Func {
    const LIBRARY_PATH_MAX_LENGTH: i32 = 4096;
    return Func::wrap(
        store.as_context_mut(),
        |mut caller: Caller<'_, WasiP1Ctx>, ptr: i32, library_len: i32| -> i32 {
            // println!("Executing wasm_dlopen..."); // TODO check here if it is already in instances
            if library_len > LIBRARY_PATH_MAX_LENGTH {
                panic!("Length of library to dlopen cannot be larger than {}!", LIBRARY_PATH_MAX_LENGTH);
            }
            let instances = get_instances();
            let instances_guard = &mut instances.instances.lock().unwrap();
            let instances:&mut Vec<Instance> = &mut *instances_guard;
            
            let global_objects = get_global_objects();
            
            let linker_guard = &mut global_objects.linker.lock().unwrap();
            let linker: &mut Linker<WasiP1Ctx> = &mut *linker_guard;

            let engine = &global_objects.engine;

            let mut library_name = get_name_from_memory(&mut caller, ptr, library_len);
            let store = caller.as_context_mut();
            
            if let Ok(eval_dir) = env::var("EVALUATION_DIR") {
                // println!("Evaluation directory: {}", eval_dir);
                library_name = eval_dir + "/" + &library_name;
            } else {
                panic!("EVALUATION_DIR environment variable not found!");
            }
            // println!("library_name: {}", &library_name);

            let module = Module::from_file(engine, &library_name).unwrap(); // TODO check if it is persisted in the OS
            let instance = linker.instantiate(store, &module).unwrap(); // TODO we need to first instantiate its requirements
            instances.push(instance);                                           //TODO error handling
            // println!("Loaded library: {}", &library_name);
            (instances.len() - 1) as i32
        },
    );
}


pub fn make_wasm_dlcall(mut store: impl AsContextMut<Data = WasiP1Ctx>) -> Func {
    const SYMBOL_MAX_LENGTH: i32 = 4096;
    const XTENSOR_CPP_ENTRY_POINT: &str = "xtensor_cpp_entry_point";
    
    return Func::wrap(
        store.as_context_mut(),
        |mut caller: Caller<'_, WasiP1Ctx>, handle: i32, symbol_ptr:i32, symbol_len:i32, buffer_size: i32| -> i32 {
            // println!("Executing dlcall function");
            
            //Safety check, valid handle to get instance
            
            let instances = get_instances();
            let instances_guard = &mut instances.instances.lock().unwrap();
            let instances:&mut Vec<Instance> = &mut *instances_guard;
            if instances.len() - 1 < (handle as usize) || handle < 0 {
                panic!("Handle index out of bounds");   
            }
            
            let instance = instances.get(handle as usize).unwrap_or_else(|| panic!("Could not unwrap instance!"));
            let option_func = instance.get_func(caller.as_context_mut(), XTENSOR_CPP_ENTRY_POINT);

            if option_func.is_none() {
                panic!("No function with name {} found!", XTENSOR_CPP_ENTRY_POINT);
            }
            let option_func = option_func
                .unwrap_or_else(|| {panic!("Could not unwrap function with name {}!", XTENSOR_CPP_ENTRY_POINT)});
            
            
            let params = [Val::I32(buffer_size)];
            let mut results:Vec<Val> = Vec::new();
            results.push(Val::I32(0));
            
            match option_func.call(caller.as_context_mut(), &params, &mut results) {
                Ok(()) => {},
                _ => panic!("Something went wrong when calling into dyn linked library!")
            };
            
            results[0].i32().unwrap_or_else(|| panic!("Could not unwrap dlcall result!"))
        },
    );
}

// dummy host function to test importing from CPython
pub fn make_wasm_dlopen2(mut store: impl AsContextMut<Data = WasiP1Ctx>) -> Func {
    
    return Func::wrap(
        store.as_context_mut(),
        | _caller: Caller<'_, WasiP1Ctx>| -> i32 {
            66
        },
    );
}
