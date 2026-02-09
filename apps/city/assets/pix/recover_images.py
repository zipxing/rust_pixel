#!/usr/bin/env python3
"""
Recover original images from .pix files + old-architecture symbols.png (1024x1024, 8x8 symbols).

Atlas layout:
  - symbols.png: 1024x1024 pixels
  - Symbol size: 8x8 pixels
  - Block: 16x16 symbols = 128x128 pixels
  - Block grid: 8 columns x 8 rows = 64 blocks
  - Each block addressed by tex_block index: col = tex_block % 8, row = tex_block // 8

Each .pix file is 8x8 cells, referencing one quadrant of a 16x16 symbol block.
Each .pix tile renders to 64x64 pixels.

Mapping: (tex_block, symbol_index) -> pixel coordinates in symbols.png
  block_col = tex_block % 8
  block_row = tex_block // 8
  sym_col   = symbol_index % 16
  sym_row   = symbol_index // 16
  pixel_x   = block_col * 128 + sym_col * 8
  pixel_y   = block_row * 128 + sym_row * 8
"""

import os
import re
import sys
import math
from collections import defaultdict

try:
    from PIL import Image
except ImportError:
    print("Need Pillow: pip3 install Pillow")
    sys.exit(1)

SYMBOL_SIZE = 8       # 8x8 pixels per symbol
BLOCK_SYMBOLS = 16    # 16x16 symbols per block
BLOCK_PIXELS = BLOCK_SYMBOLS * SYMBOL_SIZE  # 128 pixels per block
BLOCKS_PER_ROW = 8    # 1024 / 128


def tex_sym_to_pixel(tex_block, symbol_index):
    """Map (tex_block, symbol_index) to pixel coordinates in old 1024x1024 symbols.png."""
    block_col = tex_block % BLOCKS_PER_ROW
    block_row = tex_block // BLOCKS_PER_ROW
    sym_col = symbol_index % BLOCK_SYMBOLS
    sym_row = symbol_index // BLOCK_SYMBOLS
    px = block_col * BLOCK_PIXELS + sym_col * SYMBOL_SIZE
    py = block_row * BLOCK_PIXELS + sym_row * SYMBOL_SIZE
    return px, py


def parse_pix(filepath):
    """Parse a .pix file, return (width, height, cells).
    cells is list of rows, each row is list of (symbol_index, fg, tex_block, bg)."""
    with open(filepath) as f:
        lines = f.readlines()

    header = lines[0].strip()
    m = re.match(r'width=(\d+),height=(\d+),texture=(\d+)', header)
    if not m:
        return None
    w, h, tex_default = int(m.group(1)), int(m.group(2)), int(m.group(3))

    cells = []
    for line in lines[1:]:
        row = []
        for token in line.strip().split():
            parts = token.split(',')
            if len(parts) == 4:
                sym, fg, tex, bg = int(parts[0]), int(parts[1]), int(parts[2]), int(parts[3])
            elif len(parts) == 3:
                sym, fg, tex = int(parts[0]), int(parts[1]), int(parts[2])
                bg = 0
            elif len(parts) == 2:
                sym, fg = int(parts[0]), int(parts[1])
                tex = tex_default
                bg = 0
            else:
                continue
            row.append((sym, fg, tex, bg))
        if row:
            cells.append(row)
    return w, h, cells


def render_pix(symbols_img, cells):
    """Render a parsed .pix into an Image by extracting tiles from symbols.png."""
    h = len(cells)
    w = max(len(row) for row in cells) if cells else 0
    out = Image.new('RGBA', (w * SYMBOL_SIZE, h * SYMBOL_SIZE), (0, 0, 0, 0))

    for row_idx, row in enumerate(cells):
        for col_idx, (sym, fg, tex, bg) in enumerate(row):
            px, py = tex_sym_to_pixel(tex, sym)
            tile = symbols_img.crop((px, py, px + SYMBOL_SIZE, py + SYMBOL_SIZE))
            out.paste(tile, (col_idx * SYMBOL_SIZE, row_idx * SYMBOL_SIZE))

    return out


def main():
    pix_dir = os.path.dirname(os.path.abspath(__file__))
    symbols_path = os.path.join(pix_dir, 'symbols.png')
    out_dir = os.path.join(pix_dir, 'recovered')

    if not os.path.exists(symbols_path):
        print(f"Error: {symbols_path} not found")
        sys.exit(1)

    symbols_img = Image.open(symbols_path).convert('RGBA')
    print(f"Loaded symbols.png: {symbols_img.size}")

    os.makedirs(out_dir, exist_ok=True)

    # Group files by prefix
    groups = defaultdict(list)
    for f in sorted(os.listdir(pix_dir)):
        if not f.endswith('.pix'):
            continue
        m = re.match(r'([a-z]+)(\d+)\.pix', f)
        if m:
            groups[m.group(1)].append((int(m.group(2)), f))

    # Render individual tiles and save
    rendered = {}  # filename -> Image
    for f in sorted(os.listdir(pix_dir)):
        if not f.endswith('.pix'):
            continue
        filepath = os.path.join(pix_dir, f)
        result = parse_pix(filepath)
        if result is None:
            print(f"  Skip {f}: bad header")
            continue
        w, h, cells = result
        img = render_pix(symbols_img, cells)
        rendered[f] = img

    # Assemble groups into composite images
    tile_w = SYMBOL_SIZE * 8   # 64 pixels per tile
    tile_h = SYMBOL_SIZE * 8   # 64 pixels per tile

    for group, files in sorted(groups.items()):
        files.sort()
        count = len(files)
        # Choose grid layout
        cols = int(math.ceil(math.sqrt(count)))
        rows = int(math.ceil(count / cols))

        composite = Image.new('RGBA', (cols * tile_w, rows * tile_h), (0, 0, 0, 0))

        for i, (idx, fname) in enumerate(files):
            if fname not in rendered:
                continue
            tile = rendered[fname]
            cx = (i % cols) * tile_w
            cy = (i // cols) * tile_h
            composite.paste(tile, (cx, cy))

        out_path = os.path.join(out_dir, f'{group}_composite.png')
        composite.save(out_path)
        print(f"  {group}: {count} tiles -> {cols}x{rows} grid ({composite.size[0]}x{composite.size[1]}) -> {out_path}")

    # Also save individual tiles
    for fname, img in sorted(rendered.items()):
        base = os.path.splitext(fname)[0]
        img.save(os.path.join(out_dir, f'{base}.png'))

    print(f"\nDone! {len(rendered)} tiles + {len(groups)} composites saved to {out_dir}/")


if __name__ == '__main__':
    main()
