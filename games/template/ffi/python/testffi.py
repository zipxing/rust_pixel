#!/usr/bin/env python
# coding=utf-8
from cffi import FFI
from inspect import getmembers
from pprint import pprint
import json

ffi = FFI()
ffi.cdef(open('pffi.h').read())
print("Open template_ffi dylib...")
lib = ffi.dlopen("../target/release/libtemplate_ffi.dylib")

npc = lib.rs_TemplateData_new()
print(npc)
lib.rs_TemplateData_shuffle(npc)
out = ffi.new("uint8_t []", [0])
print(lib.rs_TemplateData_next(npc, out))
print(out[0])

lib.rs_TemplateData_free(npc)
