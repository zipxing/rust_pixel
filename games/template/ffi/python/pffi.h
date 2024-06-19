typedef struct rs_TemplateData rs_TemplateData;

struct rs_TemplateData *rs_TemplateData_new(void);

void rs_TemplateData_free(struct rs_TemplateData *p_pcs);

int8_t rs_TemplateData_shuffle(struct rs_TemplateData *p_pcs);

int8_t rs_TemplateData_next(struct rs_TemplateData *p_pcs, uint8_t *p_out);
