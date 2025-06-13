import importlib, sys, timeit
import time
import random
def cold_import(modname):
    sys.modules.pop(modname, None)
    return importlib.import_module(modname)

t = timeit.timeit('cold_import("math_ext")',
                  setup='from __main__ import cold_import',
                  number=1000)

print(f"Average cold import of xtensor: {t/1000:.6f} s")
from math_ext import add_lists

# Generate first 3x4 matrix
matrix1 = [random.randint(1, 9) for _ in range(12)]

# Generate second 3x4 matrix
matrix2 = [random.randint(1, 9) for _ in range(12)]
start_time = time.perf_counter()
result = add_lists(matrix2, matrix1)
end_time = time.perf_counter()
elapsed_time = end_time - start_time
print(f"Time to perform dynlib-load call was: {elapsed_time * 1000}ms\n")

start_time = time.perf_counter()
result = add_lists(matrix2, matrix1)
end_time = time.perf_counter()
elapsed_time = end_time - start_time
print(f"Time to perform dynlib-load call was: {elapsed_time * 1000}ms\n")

start_time = time.perf_counter()
result = add_lists(matrix2, matrix1)
end_time = time.perf_counter()
elapsed_time = end_time - start_time
print(f"Time to perform dynlib-load call was: {elapsed_time * 1000}ms\n")
