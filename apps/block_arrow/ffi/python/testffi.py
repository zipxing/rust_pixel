#!/usr/bin/env python
# coding=utf-8
from cffi import FFI
from inspect import getmembers
from pprint import pprint
import json

ffi = FFI()
ffi.cdef(open('pffi.h').read())
print("Open block_arrow_ffi dylib...")
lib = ffi.dlopen("../target/release/libblock_arrow_ffi.dylib")

npc = lib.rs_Block_arrowData_new()
print(npc)
lib.rs_Block_arrowData_shuffle(npc)
out = ffi.new("uint8_t []", [0])
print(lib.rs_Block_arrowData_next(npc, out))
print(out[0])

lib.rs_Block_arrowData_free(npc)
