cmake_minimum_required(VERSION 3.29)
project(math_ext LANGUAGES CXX)
find_package(pybind11 CONFIG REQUIRED)

pybind11_add_module(math_ext src/math_ext.cpp)
target_compile_features(math_ext PRIVATE cxx_std_20)
target_include_directories(math_ext
    PRIVATE
        ${PROJECT_SOURCE_DIR}/xtensor/include
        ${PROJECT_SOURCE_DIR}/xtl/include

)
install(TARGETS math_ext  DESTINATION .)
