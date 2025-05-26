use wasmtime::{AsContextMut, Caller, Func, Linker, Val, Module, Instance};
use wasmtime_wasi::WasiP1Ctx;
use crate::{get_global_objects, get_instances, get_name_from_memory};


pub fn make_wasm_dlopen(mut store: impl AsContextMut<Data = WasiP1Ctx>) -> Func {
    const LIBRARY_PATH_MAX_LENGTH: i32 = 4096;
    return Func::wrap(
        store.as_context_mut(),
        |mut caller: Caller<'_, WasiP1Ctx>, ptr: i32, library_len: i32| -> i32 {
            println!("Executing wasm_dlopen..."); // TODO check here if it is already in instances
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
            
            let library_name = get_name_from_memory(&mut caller, ptr, library_len);
            let store = caller.as_context_mut();
            
            let module = Module::from_file(engine, &library_name).unwrap(); // TODO check if it is persisted in the OS
            let instance = linker.instantiate(store, &module).unwrap(); // TODO we need to first instantiate its requirements
            instances.push(instance);
            println!("Loaded library: {}", &library_name);
            (instances.len() - 1) as i32
        },
    );
}


pub fn make_wasm_dlcall(mut store: impl AsContextMut<Data = WasiP1Ctx>) -> Func {
    const SYMBOL_MAX_LENGTH: i32 = 4096;
    return Func::wrap(
        store.as_context_mut(),
        |mut caller: Caller<'_, WasiP1Ctx>, handle: i32, ptr: i32, symbol_len: i32| -> i32 {
            println!("Executing dlcall function");
            if symbol_len > SYMBOL_MAX_LENGTH {
                panic!("Length of library to dlopen cannot be larger than {}!", SYMBOL_MAX_LENGTH);
            }
            let instances = get_instances();
            let instances_guard = &mut instances.instances.lock().unwrap();
            let instances:&mut Vec<Instance> = &mut *instances_guard;
            if instances.len() - 1 < (handle as usize) || handle < 0 {
                panic!("Handle index out of bounds");   
            }
            
            let symbol_name = get_name_from_memory(&mut caller, ptr, symbol_len);
            
            let instance = instances.get(handle as usize).unwrap_or_else(|| panic!("Could not unwrap instance!"));
            let option_func = instance.get_func(caller.as_context_mut(), &symbol_name);

            if option_func.is_none() {
                panic!("No function with name {} found!", &symbol_name);
            }
            let option_func = option_func.unwrap();

            let params = [Val::I32(177)];
            let mut results:Vec<Val> = Vec::new();
            results.push(Val::I32(0));

            match option_func.call(caller.as_context_mut(), &params, &mut results) {
                Ok(()) => {},
                _ => panic!("Could not apply mul_by_3 into function!")
            };
            
            results[0].i32().unwrap()
        },
    );
}