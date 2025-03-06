#!/usr/bin/env python
# coding=utf-8
from cffi import FFI
from inspect import getmembers
from pprint import pprint
import json

ffi = FFI()
ffi.cdef(open('pffi.h').read())
print("Open colorblk_ffi dylib...")
lib = ffi.dlopen("../target/release/libcolorblk_ffi.dylib")

npc = lib.rs_ColorblkData_new()
print(npc)
lib.rs_ColorblkData_shuffle(npc)
out = ffi.new("uint8_t []", [0])
print(lib.rs_ColorblkData_next(npc, out))
print(out[0])

lib.rs_ColorblkData_free(npc)
