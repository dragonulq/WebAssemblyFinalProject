#include <xtensor/containers/xarray.hpp>
#include <xtensor/io/xio.hpp>
#include <xtensor/views/xview.hpp>
#include <xtensor/containers/xadapt.hpp>

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <cstddef>
#include <vector>
#include <string_view>
#include <cassert>
#include <chrono>
#include <stdexcept>
#include <fstream>

extern int write_to_host_buffer(uint8_t* data, int data_len)
    __attribute__((import_module("host"), import_name("write_to_host_buffer")));

extern int read_from_host_buffer(uint8_t* data, int data_len)
    __attribute__((import_module("host"), import_name("read_from_host_buffer")));

struct Deserialized
{
    std::string_view            func_name;
    std::vector<int>            scalars;
    std::vector<xt::xarray<int>> arrays;
};

/*  buffer  – first byte of the message
 *  nbytes  – total size; the first 4 native-endian bytes in the buffer
 *            should equal this
 *
 *  Assumption:
 *     host is little-endian with 4-byte 2’s-complement int == Python’s int
 */
int write_log_to_virtual_file(const std::string& log_message) {
    // Open the file in append mode.
    const std::string LOG_FILENAME = "/timings_instance_not_alive.txt";
    std::ofstream outfile(LOG_FILENAME, std::ios_base::app);

    if (!outfile.is_open()) {
        std::cerr << "Wasm Error: Could not open virtual file " << LOG_FILENAME << " for writing." << std::endl;
        return 0;
    }

    outfile << log_message << std::endl; // Write the message followed by a newline
    outfile.close(); // Close the file
    return 1;
}

inline Deserialized deserialize(const std::uint8_t* buffer, std::size_t nbytes)
{
    constexpr std::size_t item_sz = sizeof(int);          // 4
    const std::uint8_t* cur = buffer;

    auto read_int = [&]( ) -> int
    {
        int v;
        std::memcpy(&v, cur, item_sz);
        cur += item_sz;
        return v;
    };


    int buf_size = read_int();
    assert(static_cast<std::size_t>(buf_size) == nbytes);

    const char*  name_start = reinterpret_cast<const char*>(cur);

    std::size_t max_remaining = nbytes - (cur - buffer);  // bytes left in blob

    std::size_t name_len = strnlen(name_start, max_remaining);
    std::string_view func_name{name_start, name_len};

    cur += name_len + 1;                                  // skip terminator

    int n_scalars = read_int();
    int n_arrays  = read_int();

    // read scalars
    std::vector<int> scalars;
    scalars.reserve(n_scalars);
    for (int i = 0; i < n_scalars; ++i)
        scalars.push_back(read_int());

    // vector to keep track of arrays
    std::vector<xt::xarray<int>> arrays;
    arrays.reserve(n_arrays);

    for (int a = 0; a < n_arrays; ++a)
    {
        int ndim = read_int();

        std::vector<std::size_t> shape;
        shape.reserve(ndim);
        for (int d = 0; d < ndim; ++d)
            shape.push_back(static_cast<std::size_t>(read_int()));

        // flatten length = product(shape)
        std::size_t flat_len = 1;
        for (auto s : shape) flat_len *= s;

        const int* data_ptr = reinterpret_cast<const int*>(cur);
        cur += flat_len * item_sz;

        xt::xarray<int> arr = xt::adapt(
            data_ptr,
            flat_len,
            xt::no_ownership(),
            shape
        );

        arrays.emplace_back(std::move(arr));
    }

    assert(static_cast<std::size_t>(cur - buffer) == nbytes);   // sanity check

    return {func_name, std::move(scalars), std::move(arrays)};
}

extern "C" {


int xtensor_cpp_entry_point(int buffer_size) {

    if(buffer_size <= 0) {
        return 1;
    }
    uint8_t *serialized_data = (uint8_t*) malloc(buffer_size);
    if(serialized_data == NULL) {
        fprintf(stderr, "Could not allocate memory for serialized data!\n");
        return 1;
    }
    read_from_host_buffer(serialized_data, buffer_size);

    Deserialized deserialized_data = deserialize(serialized_data, buffer_size);

    std::string func_name_str = std::string(deserialized_data.func_name);

    int result_code = 0;
    if(func_name_str == "add") {

        if (deserialized_data.arrays.size() < 2) {
                    fprintf(stderr, "Error: 'add' requires at least 2 arrays.\n");
                    result_code = -10;
                } else {

                    auto& a = deserialized_data.arrays[0];
                    auto& b = deserialized_data.arrays[1];


                    if (a.shape() != b.shape()) {
                        fprintf(stderr, "Error: Shapes of arrays are not identical for addition.\n");
                        result_code = -11;
                    } else {
                        const std::string LOG_FILENAME = "wasm_computation_log.txt";
                        auto start = std::chrono::high_resolution_clock::now();

                        xt::xarray<int> result = a + b;

                        auto end = std::chrono::high_resolution_clock::now();
                        std::chrono::nanoseconds duration_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(end - start);


                        double duration_ms = static_cast<double>(duration_ns.count()) / 1e6;
                        std::stringstream ss;
                        ss << std::fixed << std::setprecision(6) << "Xtensor computation took: " << duration_ms << " ms";
                        std::string log_message = ss.str();
                        if (write_log_to_virtual_file(log_message)) {
                            printf("Wrote to timings file from C++!\n");
                        }


                        const auto& shape = result.shape();
                        size_t data_size = result.size() * sizeof(int);
                        size_t shape_size = shape.size() * sizeof(int);


                        size_t result_buffer_size = sizeof(int) + shape_size + data_size;
                        uint8_t* result_buffer = (uint8_t*)malloc(result_buffer_size);

                        if (result_buffer == NULL) {
                             fprintf(stderr, "Error: Could not allocate memory for result buffer.\n");
                             result_code = -2;
                        } else {
                            uint8_t* cur = result_buffer;
                            int ndim = shape.size();
                            memcpy(cur, &ndim, sizeof(int));
                            cur += sizeof(int);

                            memcpy(cur, shape.data(), shape_size);
                            cur += shape_size;

                            memcpy(cur, result.data(), data_size);

                            write_to_host_buffer(result_buffer, result_buffer_size);
                            free(result_buffer);
                            return result_buffer_size;
                        }
                    }
                }

    } else if(func_name_str == "sub") {
        printf("Successfull passing of sub!\n");

    } else if(func_name_str == "mult") {
        printf("Successfull passing of mult!\n");
    }

    return result_code;
}


}

