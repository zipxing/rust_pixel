#include <stdio.h>
#include "colorblk_ffi.h"

void test_colorblk() {
    unsigned char out[1];
    rs_ColorblkData *td = rs_ColorblkData_new();
    rs_ColorblkData_shuffle(td);
    rs_ColorblkData_next(td, out);
    printf("out...%d\n", out[0]);
    rs_ColorblkData_free(td);
}

int main()
{
    test_colorblk();
    printf("\n");
    return 0;
}

