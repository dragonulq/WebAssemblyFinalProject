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
#include <stdexcept>

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

    // func name: N bytes + '\0' terminator
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

// Keep in mind a few things to check that could ruin things
// casting from size_t to int and the other way around
// passing around char* instead of uint8_t*, although it should be fine,c but add it to the list of assumptions

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

                        xt::xarray<int> result = a + b;


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

