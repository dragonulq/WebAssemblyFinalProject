use wasmtime::{AsContextMut, Caller, Func, Linker, Val, Module, Instance, Extern, AsContext};
use wasmtime_wasi;
use wasmtime_wasi::preview1::WasiP1Ctx;

use crate::{get_global_objects, get_instances, get_name_from_memory};
use crate::helpers::{read_bytes_from_module, write_bytes_to_module};

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
            instances.push(instance);                                           //TODO error handling
            println!("Loaded library: {}", &library_name);
            (instances.len() - 1) as i32
        },
    );
}


pub fn make_wasm_dlcall(mut store: impl AsContextMut<Data = WasiP1Ctx>) -> Func {
    const SYMBOL_MAX_LENGTH: i32 = 4096;
    const NUMPY_C_ENTRY_POINT: &str = "numpy_c_entry_point";
    const MAX_ARGS_SIZE: i32 = 4096 * 10; // Or some other length
    return Func::wrap(
        store.as_context_mut(),
        |mut caller: Caller<'_, WasiP1Ctx>, handle: i32, buffer_ptr: i32, buffer_size: i32| -> i32 {
            println!("Executing dlcall function");
            
            //Safety check, valid handle to get instance
            
            let instances = get_instances();
            let instances_guard = &mut instances.instances.lock().unwrap();
            let instances:&mut Vec<Instance> = &mut *instances_guard;
            if instances.len() - 1 < (handle as usize) || handle < 0 {
                panic!("Handle index out of bounds");   
            }
            
            
            let instance = instances.get(handle as usize).unwrap_or_else(|| panic!("Could not unwrap instance!"));
            let option_func = instance.get_func(caller.as_context_mut(), NUMPY_C_ENTRY_POINT);

            if option_func.is_none() {
                panic!("No function with name {} found!", NUMPY_C_ENTRY_POINT);
            }
            let option_func = option_func
                .unwrap_or_else(|| {panic!("Could not unwrap function with name {}!", NUMPY_C_ENTRY_POINT)});

            
            //Start preparing arguments for callee
            let mut backing_array = [0u8; MAX_ARGS_SIZE as usize];
            let buffer: &mut [u8] = &mut backing_array[0..buffer_size as usize];
            
            //Read args as raw bytes from caller module
            read_bytes_from_module(&mut caller, buffer, buffer_ptr);
            
            //Write bytes to callee module from host
            let mut addr_to_write;
            match instance.get_memory(caller.as_context_mut(), "memory") {
                Some(memory) => {
                    //Allocate memory with function exported from callee module
                    let alloc_func = instance
                        .get_func(caller.as_context_mut(), "wasm_alloc")
                        .unwrap_or_else(|| panic!("Could not get allocator function from calee module!"));
                    
                    //Call allocator function
                    let params = [Val::I32(buffer_size)];
                    let mut results:Vec<Val> = Vec::new();
                    results.push(Val::I32(0));
                    alloc_func.call(caller.as_context_mut(), &params, &mut results)
                        .unwrap_or_else(|err| {
                            panic!("Failed to call wasm allocator: {}", err);
                        } );
                    addr_to_write = results[0].i32().unwrap_or_else(|| {
                        panic!("Failed to get memory address from callee module!",);
                    });
                    
                    //Perform the actual write
                    write_bytes_to_module(&mut caller, memory, addr_to_write, buffer);
                },
                
                None => {panic!("Could not get memory from instance that we are trying to write to")},
            }
            
            
            let params = [Val::I32(addr_to_write)];
            let mut results:Vec<Val> = Vec::new();
            results.push(Val::I32(0));

            match option_func.call(caller.as_context_mut(), &params, &mut results) {
                Ok(()) => {},
                _ => panic!("Could not apply mul_by_3 into function!")
            };
            
            let packed_result = results[0].i64().unwrap();
            let arg_ptr = (packed_result >> 32) as i32;
            let result_size = (packed_result & 0xFFFFFFFF) as i32;

            //////////////////////////////////////////////////////////////////////////
            // Return trip - write results back to Caller Module and deserialize there
            //////////////////////////////////////////////////////////////////////////
            
            let mut backing_array = [0u8; MAX_ARGS_SIZE as usize]; // use MAX_ARGS_SIZE for result as well
            let buffer: &mut [u8] = &mut backing_array[0..result_size as usize];

            //Read args as raw bytes from callee module
            let memory =  match instance.get_export(caller.as_context_mut(),"memory") {
                Some(Extern::Memory(memory)) => memory.clone(),   // clone the reference to it
                _ => panic!("missing memory export!")
            };

            match memory.read(caller.as_context(), arg_ptr as usize, buffer) {
                Ok(()) => {},
                _ => panic!("Something went wrong while reading guest memory to get library name!")
            }
            
            match caller.get_export("memory") {
                Some(memory) => {
                    
                    //Allocate memory with function exported from caller module
                    let memory = memory.into_memory()
                        .unwrap_or_else(|| panic!("Could not convert an Extern to memory!"));
                    let alloc_func = caller.get_export("wasm_alloc")
                        .unwrap_or_else(|| panic!("Could not get alloc function from memory!"))
                        .into_func()
                        .unwrap_or_else(|| panic!("Could not get alloc function from Extern!"));

                    //Call allocator function
                    let params = [Val::I32(result_size)];
                    let mut results:Vec<Val> = Vec::new();
                    results.push(Val::I32(0));

                    alloc_func.call(caller.as_context_mut(), &params, &mut results)
                        .unwrap_or_else(|err| {
                            panic!("Failed to call wasm allocator: {}", err);
                        } );
                    addr_to_write = results[0].i32().unwrap_or_else(|| {
                        panic!("Failed to get memory address from callee module!");
                    });

                    //Perform the actual write
                    write_bytes_to_module(&mut caller, memory, addr_to_write, buffer);
                    
                },

                None => {panic!("Could not get memory from instance that we are trying to write to")},
            }
            
            addr_to_write    // Return ptr to buffer holding the serialized result
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
