#include <stdio.h>

int main()
{
    unsigned int rn = 10;
    unsigned int r = rn * 214013 + 2531011;
    r &= 0x7FFFFFFF;
    printf("Hello world %u\n", (r >> 16) & 0x7FFF);
    unsigned int a = 2147483648;
    printf("%x\n", a);
    return 0;
}

