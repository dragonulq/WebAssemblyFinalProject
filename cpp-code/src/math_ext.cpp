#include <pybind11/pybind11.h>
#include <xtensor/containers/xarray.hpp>
#include <xtensor/io/xio.hpp>
#include <xtensor/views/xview.hpp>
#include <xtensor/containers/xadapt.hpp>
#include <pybind11/pybind11.h>
#include <pybind11/stl.h>

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
namespace py = pybind11;

std::vector<double>
add_lists(const std::vector<double>& a,
          const std::vector<double>& b)
{
    if (a.size() != b.size())
        throw std::invalid_argument("lengths differ");


    xt::xarray<double> xa = xt::adapt(a);
    xt::xarray<double> xb = xt::adapt(b);


    auto res = xa + xb;


    return {res.begin(), res.end()};
}

//  Stubs to prevent unused headers from being optimized out

std::string use_xtensor_io() {
    xt::xarray<int> arr = {{1, 2, 3}, {4, 5, 6}};
    std::stringstream ss;
    ss << arr;
    return ss.str();
}


double use_xtensor_view() {
    xt::xarray<double> matrix = {{1.0, 2.0}, {3.0, 4.0}};
    auto view = xt::view(matrix, 1);
    return view(0) + view(1);
}


int64_t use_cstdint() {
    int64_t a = 1000000000;
    int64_t b = 2;
    return a * b;
}

double use_chrono() {
    auto start = std::chrono::high_resolution_clock::now();

    auto end = std::chrono::high_resolution_clock::now();
    std::chrono::duration<double, std::milli> elapsed = end - start;
    return elapsed.count();
}


std::string use_fstream(const std::string& message) {

    std::ofstream outfile("dummy_log.txt");
    if (!outfile.is_open()) {
        throw std::runtime_error("Could not open file for writing.");
    }
    outfile << message << std::endl;
    outfile.close();
    return "Message written to dummy_log.txt";
}

std::string use_string() {
    const char* part1 = "Hello, ";
    const char* part2 = "String!";
    std::string result = std::string(part1) + std::string(part2);
    return result;
}



std::vector<std::string> use_pybind11_stl(std::vector<std::string> list_in) {
    std::reverse(list_in.begin(), list_in.end());
    return list_in;
}




// Module definition
PYBIND11_MODULE(math_ext, m) {
    m.doc() = "Simple math extension";
    m.def("add_lists",      &add_lists,      "Add two 1-D Python lists");
    m.def("use_xtensor_io", &use_xtensor_io, "Stub for <xtensor/io/xio.hpp>");
    m.def("use_xtensor_view", &use_xtensor_view, "Stub for <xtensor/views/xview.hpp>");
    m.def("use_cstdint", &use_cstdint, "Stub for <cstdint>");
    m.def("use_chrono", &use_chrono, "Stub for <chrono>");
    m.def("use_fstream", &use_fstream, "Stub for <fstream>");
    m.def("use_string", &use_string, "Stub for <string>");
    m.def("use_pybind11_stl", &use_pybind11_stl, "Stub for <pybind11/stl.h>");

}
