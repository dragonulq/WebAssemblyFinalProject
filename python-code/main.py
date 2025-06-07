import importlib
import time
from pathlib import Path
from typing import Callable, List
# from host_functions import call_host
from collections.abc import Iterable
import sys

import ctypes, struct, sys

###########################################################################################
###########################################################################################


libc   = ctypes.CDLL(None)
malloc = libc.malloc
malloc.restype = ctypes.c_void_p
malloc.argtypes = (ctypes.c_size_t,)

free = libc.free
free.argtypes = (ctypes.c_void_p,)

###########################################################################################
###########################################################################################

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

###########################################################################################
###########################################################################################


def ints_to_c_buffer_pack_into(func_name, arrays, scalars):

    library_name_bytes = func_name.encode('utf-8')
    format_string = f'={len(library_name_bytes)}sx'

    scalars_no = len(scalars)
    arrays_no = len(arrays)
    item_sz  = ctypes.sizeof(ctypes.c_int)
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

    ptr = malloc(buf_size)
    if not ptr:
        raise MemoryError("malloc failed")

    #     Wrap the raw address in a ctypes *char array* that supports the
    #     Python buffer protocol and points at exactly the same memory.
    CharArray = ctypes.c_char * buf_size
    buf_view  = CharArray.from_address(ptr)


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

    return ptr, buf_size


def free_buffer(ptr):
    free(ptr)

###########################################################################################
###########################################################################################

def main():
    pass


if __name__ == "__main__":
    main()
