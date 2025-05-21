use wasmtime::{AsContextMut, Caller, Func, Linker, Val, AsContext, Module};
use wasmtime_wasi::WasiP1Ctx;
use crate::{get_global_objects, get_instances, get_instance_memory_copy};


pub fn make_wasm_dlopen(mut store: impl AsContextMut<Data = WasiP1Ctx>) -> Func {
    const LIBRARY_PATH_MAX_LENGTH: i32 = 4096;
    return Func::wrap(
        store.as_context_mut(),
        |mut caller: Caller<'_, WasiP1Ctx>, ptr: i32, library_len: i32| -> i32 {
            let global_objects = get_global_objects();
            let instances = get_instances();
            let linker_guard = &mut global_objects.linker.lock().unwrap();
            let linker: &mut Linker<WasiP1Ctx> = &mut *linker_guard;
            let memory = get_instance_memory_copy(&mut caller);
            let store = caller.as_context_mut();
            let engine = &global_objects.engine;


            let instances_guard = &mut instances.instances.lock().unwrap();
            let instances = &mut *instances_guard;

            let mut backing_array = [0u8; LIBRARY_PATH_MAX_LENGTH as usize];

            if library_len > LIBRARY_PATH_MAX_LENGTH {
                panic!("Length of library to dlopen cannot be larger than {}!", LIBRARY_PATH_MAX_LENGTH);
            }

            let buffer: &mut [u8] = &mut backing_array[0..library_len as usize];

            match memory.read(store.as_context(), ptr as usize, buffer) {
                Ok(()) => {},
                _ => panic!("Something went wrong while reading guest memory to get library name!")
            }

            let library_name = match std::str::from_utf8(&buffer[..library_len as usize]) {
                Ok(s) => s.to_string(),
                Err(_) => panic!("Could not convert buffer to string!"),
            };
            println!("Loaded library: {}", &library_name);
            let module = Module::from_file(engine, &library_name).unwrap();
            let instance = linker.instantiate(store, &module).unwrap(); // TODO we need to first instantiate its requirements
            instances.push(instance);
            let option_func = instance.get_func(caller.as_context_mut(), "mul_by_3");

            if option_func.is_none() {
                return (instances.len() - 1) as i32;
            }
            let option_func = option_func.unwrap();

            let params = [Val::I32(177)];
            let mut results:Vec<Val> = Vec::new();
            results.push(Val::I32(0));

            match option_func.call(caller.as_context_mut(), &params, &mut results) {
                Ok(()) => {},
                _ => panic!("Could not apply mul_by_3 into function!")
            };

            match results[0] {
                Val::I32(531) => {println!("Got expected value back")},
                Val::I32(x) => {println!("Got {} back", x)},
                _ => {}

            }
            println!("host_dlopen called with ptr={}, flags={}", ptr, library_len);
            (instances.len() - 1) as i32
        },
    );
}