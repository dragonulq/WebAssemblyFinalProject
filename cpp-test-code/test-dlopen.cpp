#include <cstdio>
extern "C" {

int mul_by_3(int x) {
    std::printf("Hello from dlopened cpp code!");
    return x * 3;
}

}

int main(int argc, char** argv)
{
    printf("3 + 4 = %d\n", 7);
    return 0;
}