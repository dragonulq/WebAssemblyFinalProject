#include <cstdio>
#include <cstdlib>
#include <limits>
#include <stdexcept>
#include <cstring>

int safe_size_t_to_int(size_t value) {
    if (value > static_cast<size_t>(std::numeric_limits<int>::max())) {
        throw std::overflow_error("size_t value too large for int");
    }
    return static_cast<int>(value);
}

//extern "C"
//int add(int, int)
//    __attribute__((import_module("math"), import_name("add")));


extern "C"
int wasm_dlopen(const char* path, int flags)
    __attribute__((import_module("host"), import_name("wasm_dlopen")));


int main(int argc, char** argv)
{
//    if (argc < 3) {
//            std::puts("usage: app <int> [<int> ...]");
//            return 0;
//        }


//        int acc = std::atoi(argv[1]);
//
//
//        for (int i = 2; i < argc; ++i) {
//            int val = std::atoi(argv[i]);
//            acc = add(acc, val);
//        }
//        printf("Finished adding numbers in accumulator!\n");
//        std::printf("acc is %d\n", acc);

        const char *library_name1 = "test-dlopen.wasm";
        printf("host_dlopen called without errors and the returned handle is %d!\n", wasm_dlopen(library_name1, safe_size_t_to_int(std::strlen(library_name1))));

        const char *library_name2 = "mult.wasm";
        printf("host_dlopen called without errors and the returned handle is %d!\n", wasm_dlopen(library_name2, safe_size_t_to_int(std::strlen(library_name2))));

        const char *library_name3 = "mult2.wasm";
        printf("host_dlopen called without errors and the returned handle is %d!\n", wasm_dlopen(library_name3, safe_size_t_to_int(std::strlen(library_name3))));


//        void *some_memory1 = malloc(sizeof(int) * 10);
//        if(some_memory1 != NULL) {
//            std::printf("Successfully allocated some mmemory!");
//        }

        return 0;
}
