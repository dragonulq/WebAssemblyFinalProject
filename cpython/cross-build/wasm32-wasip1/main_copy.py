import array, sys
import struct
from host_functions import call_host
from host_functions import wasm_dlopen, wasm_dlcall, write_to_host_buffer
from collections.abc import Iterable


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
    values1 = [12,3,4,5,6,6,2,324,12,1]
    values2 = [12,3,4,5,6,6,2,2,2,2,1]
    c = [12,3,4,5,6,6,1,10,49]
    d = [12,3,4,5,6,6,1,1000]

    handle = wasm_dlopen("xtensor_example.wasm")
    print("handle from wasm_dlopen should be 0 but is ",handle)
    addr, buf_size = serialize_dlcall_args("some_random_func", [values2, values1], [1,2,3,4])
    # print("addr where we can see serialized args is: " ,addr)
    # print("buf_size is: " , buf_size)
    result_len = wasm_dlcall(handle, "aa", addr)
    print("result_len in python is: ",result_len)

    # wasm_dlopen("some_str")
    # values = array.array('I', values)
    # memoryview(values)
    # values2 = array.array('I', values2)
    # memoryview(values2)
    # handle = wasm_dlopen()
    # write_args(a, b)

    # wasm_dlcall(handle, "add", )
    # list = read_results()
    # print(list)



if __name__ == "__main__":
    main()