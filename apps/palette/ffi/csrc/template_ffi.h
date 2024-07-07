#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

struct rs_TemplateData;

extern "C" {

rs_TemplateData *rs_TemplateData_new();

void rs_TemplateData_free(rs_TemplateData *p_pcs);

int8_t rs_TemplateData_shuffle(rs_TemplateData *p_pcs);

int8_t rs_TemplateData_next(rs_TemplateData *p_pcs, uint8_t *p_out);

} // extern "C"
