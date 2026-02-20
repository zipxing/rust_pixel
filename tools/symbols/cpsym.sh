#!/bin/bash
python3 generate_symbols.py --size 8192 --msdf
cp symbols_8192.png ../../apps/palette/assets/pix/symbols.png
cp symbol_map_8192.json ../../apps/palette/assets/pix/symbol_map.json
cp symbols_8192.png ../../apps/mdpt/assets/pix/symbols.png
cp symbol_map_8192.json ../../apps/mdpt/assets/pix/symbol_map.json
