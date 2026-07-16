# petii

`petii` turns an image—or experimentally, one natural-language prompt—into a
fixed-character-set PETSCII grid. The final asset contains only PETSCII glyph
IDs and palette indices. Generated reference images are temporary guidance;
they never become custom runtime tiles.

## Deterministic conversion

The historical positional interface remains supported:

```sh
cargo run -p petii -- input.png 40 25 1
```

Add deterministic whole-image optimization and a CPU-rendered preview with:

```sh
cargo run -p petii -- input.png 40 25 1 \
  --optimize --top-k 6 --preview preview.png > output.pix
```

Modes are:

- `0`: find the nearest PETSCII glyph for each block of a general image, with
  one foreground color per cell;
- `1`: extract an image that is already exact PETSCII art, including each
  cell's foreground and background colors;
- `2`: use mode `0` but exclude letters and digits.

## Experimental AI loop

Prompt-only mode generates a deliberately simple reference image, builds a
bounded PETSCII candidate pool, asks a multimodal critic to select and repair
it, and always keeps the best valid result seen so far:

```sh
export PETII_AI_API_KEY=...
cargo run -p petii -- ai "a moon witch above a ruined observatory" \
  --output-dir tmp/moon-witch
```

Use a supplied reference to skip image generation:

```sh
cargo run -p petii -- ai "a readable moon witch silhouette" \
  --input reference.png --output-dir tmp/moon-witch
```

Generate the reference image but run only the original top-1 PETSCII matcher,
without top-K optimization or multimodal critique:

```sh
cargo run -p petii -- ai "a moonlit lion guarding ruins" \
  --direct --output-dir tmp/lion-direct
```

With both `--direct` and `--input`, no provider or API key is required.

Use the deterministic pipeline without any provider call:

```sh
cargo run -p petii -- ai "offline study" --input reference.png \
  --offline --output-dir tmp/offline-study
```

Options include `--width`, `--height`, `--mode`, `--top-k`, `--candidates`,
`--iterations`, `--seed`, and `--output-dir`. The default width is 40; when
`--height` is omitted, rows are calculated from the actual reference-image
aspect ratio (a square reference becomes 40×40). Supplying `--height` overrides
automatic aspect preservation. Other defaults are six glyph alternatives, four
candidates, and four loop iterations.

Direct mode defaults to mode `0`, the general nearest-glyph converter. AI and
offline optimization default to mode `2`, keeping the same nearest-glyph
algorithm while restricting the candidate vocabulary to graphic symbols. Mode
`1` is reserved for extracting sources that are already exact PETSCII art.
Mode `2` removes forbidden glyph IDs from matching rather than replacing their
templates, and flat cells near the detected scene background become true space
glyphs rendered with that background color. Other flat cells become solid-block
glyphs in their locally quantized color; structural matching is reserved for
internally varying cells. Strong-edge cells use a whole-image Sobel map, local
foreground/background fill masks, and glyph-edge overlap to prefer PETSCII
half-block, diagonal, and line-like contours. Weak edge components disconnected
from a strong contour are removed before cells are classified. Edge cells keep
a bounded candidate set and select glyphs with penalties for mismatched neighbor
borders, single-sided short spurs, and thin branches that terminate inside a
3×3-cell neighborhood.

Provider configuration is read only from the environment:

- `PETII_AI_API_KEY` (required for live mode)
- `PETII_AI_API_BASE` (default `https://api.openai.com/v1`)
- `PETII_AI_IMAGE_MODEL` (default `gpt-image-2`)
- `PETII_AI_VISION_MODEL` (default `gpt-4.1-mini`)

The adapter expects OpenAI-compatible image-generation and chat-completions
response shapes. Keys are never written to output or logged.

Each run emits `final.pix`, `final.png`, `reference.png`, individual candidate
`.pix`/PNG files, `gallery.png`, `critique.json`, and a redacted
`manifest.json`. Given the same input image and conversion settings, the Rust
candidate/optimizer path is deterministic. Live reference generation and
critic behavior can still vary by provider; recorded-response replay is not
implemented yet.

## Current limitations

- This milestone targets static PETSCII scenes, not animation or game code.
- AI repair is restricted to validated regions, palette roles, density,
  contrast, silhouette protection, and individual cells. Crop repair is
  validated but deferred because it requires rebuilding the candidate pool.
- Deterministic scores measure reconstruction and PETSCII structure; they do
  not replace blinded human evaluation of artistic quality.
