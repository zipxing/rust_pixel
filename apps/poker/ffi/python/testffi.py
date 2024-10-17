#!/usr/bin/env python
# coding=utf-8
from cffi import FFI
from inspect import getmembers
from pprint import pprint
import json

# def cdata_dict(cd):
#     if isinstance(cd, ffi.CData):
#         try:
#             return ffi.string(cd)
#         except TypeError:
#             try:
#                 return [cdata_dict(x) for x in cd]
#             except TypeError:
#                 return {k: cdata_dict(v) for k, v in getmembers(cd)}
#     else:
#         return cd

ffi = FFI()
ffi.cdef(open('pffi.h').read())
print("Open poker_ffi dylib...")
lib = ffi.dlopen("../target/release/libpoker_ffi.dylib")

npc = lib.rs_PokerCards_new()

print(npc)

lib.rs_PokerCards_assign(npc, ffi.new("uint16_t []", [1, 2, 17]), 3)
counter = lib.rs_PokerCards_get_counter(npc, 0)

print(counter)
print(counter.n)

lib.rs_PokerCards_free(npc)
# # test double free 
# lib.rs_PokerCards_free(npc)

tps = lib.rs_TexasCards_new()
lib.rs_TexasCards_assign(tps, ffi.new("uint16_t []", [1, 13, 12, 11, 10, 14, 17]), 7)
bs = lib.rs_TexasCards_get_best(tps)
print(bs.cardbuf.len)
print(bs.cardbuf.data[0].suit, bs.cardbuf.data[0].number)
print(bin(bs.score))
lib.rs_TexasCardBuffer_free(bs)

lib.rs_TexasCards_assign(tps, ffi.new("uint16_t []", [1 + 13, 2 + 13, 3 + 13, 4 + 13, 5 + 13, 14 + 13, 17 + 13]), 7)
bs = lib.rs_TexasCards_get_best(tps)
print(bs.cardbuf.len)
print(bs.cardbuf.data[0].suit, bs.cardbuf.data[0].number)
print(bin(bs.score))
lib.rs_TexasCardBuffer_free(bs)

lib.rs_TexasCards_free(tps)

gcs = lib.rs_GinRummyCards_new()
out = ffi.new("uint8_t [64]")
lib.rs_GinRummyCards_assign(gcs, ffi.new("uint16_t []", [1,45, 2,3,4,5,31,32,33,40]), 10, 1, out)
print("ooooooooooooo", out[0], out[1])
lib.rs_GinRummyCards_free(gcs)

