#include <stdio.h>
#include "block_arrow_ffi.h"

void test_block_arrow() {
    unsigned char out[1];
    rs_Block_arrowData *td = rs_Block_arrowData_new();
    rs_Block_arrowData_shuffle(td);
    rs_Block_arrowData_next(td, out);
    printf("out...%d\n", out[0]);
    rs_Block_arrowData_free(td);
}

int main()
{
    test_block_arrow();
    printf("\n");
    return 0;
}

