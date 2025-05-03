#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <wasm.h>
#include <wasmtime.h>
#include <unistd.h>
#include <limits.h>
#include <sys/stat.h>

static void exit_with_error(const char *message, wasmtime_error_t *error,
                            wasm_trap_t *trap);

static wasm_trap_t *hello_callback(void *env, wasmtime_caller_t *caller,
                                   const wasmtime_val_t *args, size_t nargs,
                                   wasmtime_val_t *results, size_t nresults) {
  printf("Calling back...\n");
  printf("> Hello World From WASM module!\n");
  return NULL;
}

int main(int argc, char *argv[]) {
  int ret = 0;

  char binary_path[PATH_MAX];
  struct stat st;

  if (realpath(argv[1], binary_path) == NULL) {
    perror(".wasm binary cannot be found anymore!");
    exit(EXIT_FAILURE);
  }

  if (stat(binary_path, &st) != 0) {
    perror("stat failed");
    exit(EXIT_FAILURE);
  }

  printf("Initializing...\n");
  wasm_engine_t *engine = wasm_engine_new();
  assert(engine != NULL);
  for (int i = 0;i < argc;i++) {
    printf("Arg %d: %s\n", i, argv[i]);
  }

  wasmtime_store_t *store = wasmtime_store_new(engine, NULL, NULL);
  assert(store != NULL);
  wasmtime_context_t *context = wasmtime_store_context(store);

  FILE *file = fopen(binary_path, "rb");
  assert(file != NULL);

  wasm_byte_vec_t wasm;
  wasm_byte_vec_new_uninitialized(&wasm, st.st_size);
  if (fread(wasm.data, 1, st.st_size, file) != st.st_size) {
    perror("fread failed");
    fclose(file);
    wasm_byte_vec_delete(&wasm);
    exit(EXIT_FAILURE);
  }


  printf("Compiling module...\n");
  wasmtime_module_t *module = NULL;
  wasmtime_error_t *error = wasmtime_module_new(engine, (uint8_t *)wasm.data, wasm.size, &module);
  wasm_byte_vec_delete(&wasm);
  if (error != NULL)
    exit_with_error("failed to compile module", error, NULL);

  printf("Creating callback...\n");
  wasm_functype_t *hello_ty = wasm_functype_new_0_0();
  wasmtime_func_t hello;
  wasmtime_func_new(context, hello_ty, hello_callback, NULL, NULL, &hello);

  printf("Instantiating module...\n");
  wasm_trap_t *trap = NULL;
  wasmtime_instance_t instance;
  wasmtime_extern_t import;
  import.kind = WASMTIME_EXTERN_FUNC;
  import.of.func = hello;
  error = wasmtime_instance_new(context, module, &import, 1, &instance, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to instantiate", error, trap);

  printf("Extracting export...\n");
  wasmtime_extern_t run;
  bool ok = wasmtime_instance_export_get(context, &instance, "run", 3, &run);
  assert(ok);
  assert(run.kind == WASMTIME_EXTERN_FUNC);

  printf("Calling export...\n");
  error = wasmtime_func_call(context, &run.of.func, NULL, 0, NULL, 0, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call function", error, trap);

  printf("All finished!\n");
  ret = 0;

  wasmtime_module_delete(module);
  wasmtime_store_delete(store);
  wasm_engine_delete(engine);
  return ret;
}

static void exit_with_error(const char *message, wasmtime_error_t *error,
                            wasm_trap_t *trap) {
  fprintf(stderr, "error: %s\n", message);
  wasm_byte_vec_t error_message;
  if (error != NULL) {
    wasmtime_error_message(error, &error_message);
    wasmtime_error_delete(error);
  } else {
    wasm_trap_message(trap, &error_message);
    wasm_trap_delete(trap);
  }
  fprintf(stderr, "%.*s\n", (int)error_message.size, error_message.data);
  wasm_byte_vec_delete(&error_message);
  exit(EXIT_FAILURE);
}
