#include <stdio.h>
#include "template_ffi.h"

void test_template() {
    unsigned char out[1];
    rs_TemplateData *td = rs_TemplateData_new();
    rs_TemplateData_shuffle(td);
    rs_TemplateData_next(td, out);
    printf("out...%d\n", out[0]);
    rs_TemplateData_free(td);
}

int main()
{
    test_template();
    printf("\n");
    return 0;
}

