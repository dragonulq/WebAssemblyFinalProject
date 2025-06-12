import array, sys
import struct
from host_functions import call_host
from host_functions import wasm_dlopen, wasm_dlcall, write_to_host_buffer
from collections.abc import Iterable
import random


#############################################
#############################################


def deserialize_xtensor_result(buffer: bytes):

    cursor = 0

    try:
        (ndim,) = struct.unpack_from('<i', buffer, cursor)
        cursor += 4
    except struct.error:
        raise ValueError("Buffer is too small to contain ndim.")


    shape_format = f'<{ndim}I'
    shape_size = struct.calcsize(shape_format)

    try:
        shape = struct.unpack_from(shape_format, buffer, cursor)
        cursor += shape_size
    except struct.error:
        raise ValueError("Buffer is too small to contain the shape.")


    num_elements = 1
    for dim_size in shape:
        num_elements *= dim_size

    data_format = f'<{num_elements}i'

    try:
        flat_data = list(struct.unpack_from(data_format, buffer, cursor))
    except struct.error:
        raise ValueError("Buffer has incorrect data size for the given shape.")

    if not shape:
        return []


    def reshape_recursive(data_iterator, current_shape):

        if not current_shape:

            return next(data_iterator)


        first_dim, rest_of_shape = current_shape[0], current_shape[1:]

        return [
            reshape_recursive(data_iterator, rest_of_shape)
            for _ in range(first_dim)
        ]

    return reshape_recursive(iter(flat_data), shape)

#############################################
#############################################

def get_shape_and_flatten(items):

    shape = []
    flat_data = []

    def _traverse(sub_array, level):
        # This check is still useful to ensure we start with a list-like structure
        if not isinstance(sub_array, Iterable) or isinstance(sub_array, (str, bytes)):
            raise TypeError("Input must be a list of numbers.")

        dim_len = len(sub_array)

        if len(shape) == level:
            shape.append(dim_len)

        if dim_len > 0:
            first_item = sub_array[0]
            # If the first element is another list, recurse. Otherwise, flatten.
            if isinstance(first_item, Iterable) and not isinstance(first_item, (str, bytes)):
                for item in sub_array:
                    _traverse(item, level + 1)
            else:
                flat_data.extend(sub_array)

    if items:
        _traverse(items, 0)
    return shape, flat_data

def serialize_dlcall_args(func_name, arrays, scalars):

    library_name_bytes = func_name.encode('utf-8')
    format_string = f'={len(library_name_bytes)}sx'

    scalars_no = len(scalars)
    arrays_no = len(arrays)
    item_sz  = 4       #sizeof(int) in C
    buf_size = 3 * item_sz + len(library_name_bytes) + 1    # first elem is buffer len, next 2 elems are ints that say how many scalars and arrays are present
    buf_size += scalars_no * item_sz
    flattened_arrays = []
    original_shapes = []

    for array in arrays:
        shape, flattened_values = get_shape_and_flatten(array)
        flattened_arrays.append(flattened_values)
        original_shapes.append(shape)
        buf_size += item_sz * len(flattened_values)    # actual elements in the array
        buf_size += item_sz                            # scalar to specify how many dimensions
        buf_size += item_sz * len(shape)

    buf_view = bytearray(buf_size)

    cursor = 0
    struct.pack_into('i', buf_view, cursor, buf_size)
    cursor += item_sz

    struct.pack_into(format_string, buf_view, cursor, library_name_bytes)
    cursor += len(library_name_bytes) + 1

    struct.pack_into('2i', buf_view, cursor, scalars_no, arrays_no)
    cursor += item_sz * 2

    struct.pack_into(f'={scalars_no}i', buf_view, cursor, *scalars)
    cursor += item_sz * scalars_no

    for (flat_array, shape) in zip(flattened_arrays, original_shapes):
        dimensions = len(shape)
        struct.pack_into('i', buf_view, cursor, dimensions)
        cursor += item_sz

        struct.pack_into(f'={dimensions}i', buf_view, cursor, *shape)
        cursor += item_sz * dimensions

        struct.pack_into(f'={len(flat_array)}i', buf_view, cursor, *flat_array)
        cursor += item_sz * len(flat_array)

    if cursor != buf_size:
        print("Struct packing logic is broken, cursor != buf_size at the end")
        sys.exit(1)
    return memoryview(buf_view)[:cursor] , buf_size



#############################################
#############################################


def main():
    values1 = [0,12,3,4,5,6,6,2,324,12,1]
    values2 = [12,3,4,5,6,6,2,2,2,2,1]
    # values1 + values2
    # [12, 15, 7, 9, 11, 12, 8, 4, 326, 14, 2]


    # Generate first 3x4 matrix
    matrix1 = [[random.randint(1, 9) for _ in range(4)] for _ in range(3)]

    # Generate second 3x4 matrix
    matrix2 = [[random.randint(1, 9) for _ in range(4)] for _ in range(3)]

    # Print both matrices with nice alignment
    print("Matrix 1:")
    for row in matrix1:
        print(" ".join(f"{num:2d}" for num in row))

    print("\nMatrix 2:")
    for row in matrix2:
        print(" ".join(f"{num:2d}" for num in row))

    handle = wasm_dlopen("xtensor_adapter.wasm")
    addr, buf_size = serialize_dlcall_args("add", [matrix1, matrix2], [])
    result_raw_bytes = wasm_dlcall(handle, "add", addr)
    result_list = deserialize_xtensor_result(result_raw_bytes)

    print("\nResult of adding mattrices:")
    for row in result_list:
        print(" ".join(f"{num:2d}" for num in row))
    print("\n")


if __name__ == "__main__":
    main()