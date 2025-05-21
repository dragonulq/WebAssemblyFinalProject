#include <cstdio>

extern "C" int multp2(int a, int b) {
    return a * b * 2;
}

int main(int argc, char** argv)
{
    std::printf("3 + 4 = %d\n", 7);
    return 0;
}
