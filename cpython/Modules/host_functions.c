#include <Python.h>
#if defined(__wasi__) || defined(__wasm__) || defined(__wasm32__)



extern int wasm_dlopen2()
    __attribute__((import_module("host"), import_name("wasm_dlopen2")));
    // Or, if you want to align with common WASI practices for simple external functions,
    // some people might use "env" as the module, but it's clearer to have your own.
    // e.g., __attribute__((import_module("env"), import_name("host_side_function")));


// 2. Create a Python-callable wrapper function that uses the host function
static PyObject* ext_call_host_function(PyObject* self, PyObject* args) {
//    int inputValue;
    int host_result;
//
//    if (!PyArg_ParseTuple(args, "i", &inputValue)) {
//        return NULL; // Error in parsing arguments
//    }
//
//    // Call the imported host function
//    // The WASM runtime will resolve this call to the function provided by the host.
    host_result = wasm_dlopen2();

    return PyLong_FromLong(666);
}

// 3. Method definition table for the extension module
static PyMethodDef HostExtensionMethods[] = {
    {"call_host", ext_call_host_function, METH_VARARGS, "Calls a function defined by the WASM host."},
    {NULL, NULL, 0, NULL}        /* Sentinel */
};

// 4. Module definition structure
static struct PyModuleDef hostextensionmodule = {
    PyModuleDef_HEAD_INIT,
    "host_functions",   // Name of module
    "Module calling host WASM functions via WASI SDK attributes", // Module documentation
    -1,                 // Size of per-interpreter state of the module
    HostExtensionMethods
};

// 5. Module initialization function
PyMODINIT_FUNC PyInit_host_functions(void) {
    return PyModule_Create(&hostextensionmodule);
}
#else // --- NOT WASI/WASM build: This is for the NATIVE Host Tool Build (e.g., _freeze_module)

// ==================================================
// BEGIN --- Code for the NATIVE Host Tool Build
// ==================================================

static PyMethodDef HostFunctionsMethods_NativeStub[] = {
    {NULL, NULL, 0, NULL}
};

static struct PyModuleDef hostfunctionsmodule_NativeStub = {
    PyModuleDef_HEAD_INIT,
    "host_functions", // Module name MUST match Modules/Setup and PyInit_ name
    "Native stub for host_functions WASM module (not functional).",
    -1,
    HostFunctionsMethods_NativeStub
};

PyMODINIT_FUNC PyInit_host_functions(void) { // Name must match Modules/Setup
    PyObject *m = PyModule_Create(&hostfunctionsmodule_NativeStub);
    // Optional: You could add a specific attribute or raise an error if this stub
    // were ever actually imported and used in the native context,
    // though _freeze_module usually just needs to link against PyInit_ symbols.
    // if (m) PyModule_AddStringConstant(m, "__stub__", "This is a native stub for a WASM module.");
    return m;
}

// ==================================================
// END --- Code for the NATIVE Host Tool Build
// ==================================================

#endif // End of defined(__wasi__) || defined(__wasm__) || defined(__wasm32__)}
