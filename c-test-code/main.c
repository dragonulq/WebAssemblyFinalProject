#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

int wasm_dlopen(const char* path, int path_length)
    __attribute__((import_module("host"), import_name("wasm_dlopen")));

int wasm_dlcall(int handle, const char* symbol, int symbol_length)
    __attribute__((import_module("host"), import_name("wasm_dlcall")));

void *wasm_alloc(int bytes) {
    if(bytes < 0) {
        perror("bytes to wasm_alloc was smaller than 0!");
        exit(EXIT_FAILURE);
    }
    size_t bytes_to_alloc = (size_t) bytes;
    void *result = malloc(bytes_to_alloc);
    if(result == NULL) {
        perror("Could not allocate guest memory through wasm_alloc!");
        exit(EXIT_FAILURE);
    }
    return result;

}


int main(int argc, char** argv) {



    return 0;
}
