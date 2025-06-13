#include <Python.h>
#if defined(__wasi__) || defined(__wasm__) || defined(__wasm32__)

#include <stdlib.h>
#include <stdio.h>
#include <string.h>


extern int wasm_dlopen(const char* path, int path_length)
    __attribute__((import_module("host"), import_name("wasm_dlopen")));

extern int wasm_dlcall(int handle, const uint8_t** symbol, int symbol_length, int args_buffer_len)
    __attribute__((import_module("host"), import_name("wasm_dlcall")));

extern int write_to_host_buffer(uint8_t* data, int data_len)
    __attribute__((import_module("host"), import_name("write_to_host_buffer")));

extern int read_from_host_buffer(uint8_t** data, int data_len)
    __attribute__((import_module("host"), import_name("read_from_host_buffer")));


static PyObject* ext_call_host_function(PyObject* self, PyObject* args) {

    int host_result;



    return PyLong_FromLong(666);
}

static PyObject* ext_call_wasm_dlopen(PyObject* self, PyObject* args) {
    const char* str_data;
    Py_ssize_t str_len;

    if (!PyArg_ParseTuple(args, "s#", &str_data, &str_len)) {
        return NULL;
    }

    return PyLong_FromLong(wasm_dlopen(str_data, (int) str_len));
}


//////////////////////////////////////////
////////////wasm_dlcall///////////////////
//////////////////////////////////////////



static PyObject* ext_call_wasm_dlcall(PyObject* self, PyObject* args) {
    int handle;
    const uint8_t* symbol;
    Py_buffer args_buffer;

    if (!PyArg_ParseTuple(args, "isy*", &handle, &symbol, &args_buffer)) {
        perror("Could not parse PyObjects!");
        exit(EXIT_FAILURE);

    }
    int symbol_len = strlen(symbol);
    void* data_ptr = args_buffer.buf;
    Py_ssize_t args_buffer_len = args_buffer.len;

     Make absolutely sure that the size fits in an int
    assert(args_buffer_len <= INT_MAX && args_buffer_len >= INT_MIN);
    int data_len = (int) args_buffer_len;
    write_to_host_buffer(data_ptr, data_len);
    int result_length_in_bytes = wasm_dlcall(handle, symbol, symbol_len, data_len);
    assert(result_length_in_bytes > 0);

    PyObject* result_bytes_obj = PyBytes_FromStringAndSize(NULL, result_length_in_bytes);
    if (result_bytes_obj == NULL) {
        perror("Could not allocate buffer for storing result!");
        exit(EXIT_FAILURE);

    }
    uint8_t* result_buffer = (uint8_t*)PyBytes_AS_STRING(result_bytes_obj);
    read_from_host_buffer(result_buffer, result_length_in_bytes);

    return result_bytes_obj;
}


static PyObject* ext_write_to_host_buffer(PyObject* self, PyObject* args) {


        return PyLong_FromLong(666);
}







// 3. Method definition table for the extension module
static PyMethodDef HostExtensionMethods[] = {
    {"call_host", ext_call_host_function, METH_VARARGS, "Calls a function defined by the WASM host."},
    {"wasm_dlcall", ext_call_wasm_dlcall, METH_VARARGS, "Calls a function that writes to the WASM host buffer"},
    {"wasm_dlopen", ext_call_wasm_dlopen, METH_VARARGS, "Calls a function that writes to the WASM host buffer"},
    {"write_to_host_buffer", ext_write_to_host_buffer, METH_VARARGS, "Calls a function that writes to the WASM host buffer"},
    {NULL, NULL, 0, NULL}
};

// 4. Module definition structure
static struct PyModuleDef hostextensionmodule = {
    PyModuleDef_HEAD_INIT,
    "host_functions",   // Name of module
    "Module calling host WASM functions via WASI SDK attributes", // Module documentation
    -1,
    HostExtensionMethods
};

// 5. Module initialization function
PyMODINIT_FUNC PyInit_host_functions(void) {
    return PyModule_Create(&hostextensionmodule);
}
#else 

// ==================================================
// BEGIN --- Code for the NATIVE Host Tool Build
// ==================================================

static PyMethodDef HostFunctionsMethods_NativeStub[] = {
    {NULL, NULL, 0, NULL}
};

static struct PyModuleDef hostfunctionsmodule_NativeStub = {
    PyModuleDef_HEAD_INIT,
    "host_functions", 
    "Native stub for host_functions WASM module (not functional).",
    -1,
    HostFunctionsMethods_NativeStub
};

PyMODINIT_FUNC PyInit_host_functions(void) { 
    PyObject *m = PyModule_Create(&hostfunctionsmodule_NativeStub);
    
    return m;
}

// ==================================================
// END --- Code for the NATIVE Host Tool Build
// ==================================================

#endif // End of defined(__wasi__) || defined(__wasm__) || defined(__wasm32__)}
