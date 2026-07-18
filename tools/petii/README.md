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
- `2`: use mode `0` with local foreground/background colors while excluding
  letters, digits, and a small set of text-like punctuation glyphs.

### Deterministic conversion pipeline

The image-to-grid path has five stages:

1. **Normalize once**: apply the requested contrast, resize to exactly 8×8
   source pixels per output cell, detect the dominant scene background, and
   build a cleaned whole-image Sobel map.
2. **Generate cell candidates**: send uniform scene-background cells directly
   to space, other uniform cells to a solid block, and rank only internally
   varying cells against the fixed PETSCII charset. Mode 2 uses local two-color
   quantization so dark objects do not inherit the scene background color.
3. **Refine edge continuity**: for strong-edge cells only, merge appearance
   Top-K with a bounded set of topology-compatible PETSCII glyphs, solve stable
   contour chains and junctions, then apply generic continuity and spur repair.
   A deterministic Pareto rollback uses intermediate candidates to keep every
   accepted result within 5% of its own Top-1 reference loss. There are no
   scene-specific or shape-specific repair passes.
4. **Reference-constrained repaint**: keep the final glyphs fixed, refit their
   actual foreground/background bitmap regions with RustPixel's Lab-based
   CIEDE2000 palette distance, and reduce only boundary color jumps not present
   in the reference.
5. **Materialize the grid**: place the selected candidate first, truncate saved
   alternatives to the requested top-K bound, validate the typed grid, and emit
   `.pix`/PNG artifacts.

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

Generate the reference image and run a single enhanced conversion (slope and
dither, no optimizer or multimodal critique). This is the best full-chain
prompt-to-art path in one command:

```sh
cargo run -p petii -- ai "a moonlit lion guarding ruins" \
  --direct --width 60 --output-dir tmp/lion-direct
```

Add `--no-slopes` or `--no-dither` to turn off either enhancement. With both
`--direct` and `--input`, no provider or API key is required.

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

Direct, AI, and offline modes all default to mode `2`, the graphic-vocabulary
art path (letters, digits, and text-like punctuation excluded). Direct mode
additionally applies the slope and dither enhancements by default. Mode `1` is
reserved for extracting sources that are already exact PETSCII art.
Mode `2` removes forbidden glyph IDs from matching rather than replacing their
templates. Flat cells near the detected scene background become true space
glyphs rendered with that background color, while other flat cells become
solid-block glyphs in their locally quantized color. Internally varying cells
use local foreground/background colors: the dominant scene background is used
only when it is present in that cell. Strong-edge cells use the generic
continuity refinement described above.

Provider configuration is read only from the environment:

- `PETII_AI_API_KEY` (required for live mode)
- `PETII_AI_API_BASE` (default `https://api.openai.com/v1`)
- `PETII_AI_IMAGE_MODEL` (default `gpt-image-2`)
- `PETII_AI_VISION_MODEL` (default `gpt-4.1-mini`)

The adapter expects OpenAI-compatible image-generation and chat-completions
response shapes. Keys are never written to output or logged.

Each run emits `final.pix`, `final.png`, `reference.png`, individual candidate
`.pix`/PNG files, `gallery.png`, `critique.json`, `edge-metrics.json`, an
optional `edge-debug.png`, and a redacted `manifest.json`.

`edge-metrics.json` compares the local Top-1 baseline, the edge-grammar
proposal, and the quality-gated final selection. It records reference loss,
target-port loss, shared-border breaks, endpoint errors, contour coverage,
false junctions, spur cells, edits, chain/loop/junction counts, and the gate
decision. `edge-debug.png` uses cyan for reference ports, green for aligned
selected ports/connections, red for breaks, orange for edited cells, yellow for
spur cells, and purple for junction cells.

Given the same input image and conversion settings, the Rust
candidate/optimizer path and edge diagnostics are deterministic. Live
reference generation and critic behavior can still vary by provider;
recorded-response replay is not implemented yet.

## Deterministic benchmark

Compare local Top-1 Mode 2 conversion against the current bounded candidate,
contour, cleanup, and repaint pipeline without making provider calls:

```sh
cargo run -p petii --release -- benchmark \
  tools/petii/benchmark/v1/prompts.json \
  --reference-dir tools/petii/benchmark/v1/references \
  --output-dir tmp/petii-benchmark-v1
```

The manifest supplies case IDs, categories, prompts, and a default grid. A case
may include a `reference` path; otherwise the runner looks for
`<reference-dir>/<case-id>.png|jpg|jpeg|webp`. Options include `--width`,
`--height`, `--mode`, `--baseline-top-k`, `--candidate-top-k`, and
`--preview-scale`.

Each case directory contains `reference.png`, `baseline.pix/png`,
`candidate.pix/png`, and `metrics.json`. The root `report.json` records
candidate wins, ties, baseline wins, win-or-tie rate, mean scores, and mean
relative improvement. Reports omit timing and machine-specific absolute paths
so identical inputs and settings produce deterministic metrics.

The report also records a second, perception-aligned winner. Alongside the
per-pixel reconstruction score, each case is scored with an eye-averaged tone
distance (mean CIEDE2000 over half-glyph blocks). On the recorded blinded human
A/B this perceptual score tracks human preference roughly three times better
than reconstruction (60% vs 20% agreement), so `report.json` reports both
`win_or_tie_rate` and `perceptual_win_or_tie_rate`.

## Slope and dither enhancements

The `petii <image>` command applies two quality enhancements by default; the
`convert_image_styled(image, config, dither, slopes)` entry point toggles them
programmatically.

**Slopes** (`--no-slopes` to disable) draw diagonal silhouette boundaries with a
fill-boundary glyph family instead of stair-stepped solid blocks. A curated
catalog of horizontal fills (fill the bottom or top *k* rows), vertical fills
(left or right *k* columns), and the four 45-degree triangles is searched for
the glyph that best reproduces each silhouette cell. Shallow slopes resolve to a
stepped run of horizontal fills, steep ones to vertical fills, and 45-degree ones
to triangles, so a hill or a tower roof reads as a continuous edge. Only clean
bilevel boundaries qualify; thin lines and textured cells fall through to the
general matcher.

**Dithering** (`--no-dither` to disable) recovers intermediate tones. Flat
mid-tone cells that would otherwise collapse into a single solid block instead
pick two bracketing palette colors and a fine checker/hatch glyph whose fill
approximates the blend, the way hand-drawn PETSCII shades skies and gradients. It
is deliberately restrained: dark cells stay solid so silhouettes read, and only
cells whose single nearest palette color leaves visible banding are dithered.

The two enhancements act on disjoint cell types (silhouette boundaries versus
flat interiors), so they compose without conflict.

### Generating conversion-friendly references

AI reference generation asks for exactly the properties the converter renders
well and forbids what it cannot. The art direction requests crisp continuous
contours, large coherent color regions, 6-8 solid colors, and a few intentional
diagonal and curved edges, while avoiding text, gradients, noise, dithering,
halftone, tiny details, and pixel art. Feeding the converter a clean flat-color
illustration — not an already-textured image — is the single biggest lever on
output quality.

#### Benchmark v1 imagegen prompts

The five fixed v1 reference images were generated independently with Codex's
built-in `imagegen` skill. The exact prompts are retained below so the art
direction can be reviewed or reused. The checked-in PNG files remain the
deterministic benchmark inputs; regenerating from these prompts may produce
different images and must not overwrite v1 in place.

Portrait — `portrait-witch.png`:

```text
Use case: stylized-concept
Asset type: versioned PETSCII conversion benchmark reference, portrait category
Primary request: A solitary young witch in a broad pointed hat, head-and-shoulders portrait, facing three-quarter left.
Scene/backdrop: plain deep indigo night backdrop with one pale crescent moon, no extra objects.
Style/medium: clean flat-color retro editorial illustration, not pixel art and not PETSCII.
Composition/framing: 8:5 landscape canvas; large centered silhouette; generous outer margin; readable at 40x25 cells.
Lighting/mood: strong moonlit rim light, calm and mysterious.
Color palette: 6 to 8 solid colors, high contrast but harmonious.
Constraints: crisp continuous contours; large coherent color regions; a few intentional diagonal and curved edges; no tiny details.
Avoid: text, watermark, frame, gradients, noise, dithering, halftone, photorealism, pixel art, ASCII art, PETSCII.
```

Object — `object-airship.png`:

```text
Use case: stylized-concept
Asset type: versioned PETSCII conversion benchmark reference, object category
Primary request: One whimsical retro airship with a large oval balloon and a small gondola, side view.
Scene/backdrop: clear blue sky with only two broad simple cloud bands.
Style/medium: clean flat-color retro editorial illustration, not pixel art and not PETSCII.
Composition/framing: 8:5 landscape canvas; airship spans most of the width; complete silhouette with generous margin; readable at 40x25 cells.
Lighting/mood: bright daylight, adventurous.
Color palette: 6 to 8 solid colors, coherent sky blue dominant background.
Constraints: crisp continuous balloon and hull contours; large color regions; clear ropes and tail as simplified edge features.
Avoid: text, watermark, frame, gradients, noise, dithering, halftone, photorealism, pixel art, ASCII art, PETSCII.
```

Scene — `scene-forest.png`:

```text
Use case: stylized-concept
Asset type: versioned PETSCII conversion benchmark reference, natural scene category
Primary request: A quiet forest clearing with three prominent pine trees, a winding stream, and a pale sun behind distant hills.
Scene/backdrop: layered forest landscape.
Style/medium: clean flat-color retro editorial illustration, not pixel art and not PETSCII.
Composition/framing: 8:5 landscape canvas; clear foreground, middle ground, background layers; readable at 40x25 cells.
Lighting/mood: early morning, serene.
Color palette: 7 to 9 solid colors, deep green foreground and muted blue distance.
Constraints: crisp connected tree silhouettes; stream edge forms one continuous S curve; large coherent color regions.
Avoid: text, watermark, frame, gradients, noise, individual leaves, dithering, halftone, photorealism, pixel art, ASCII art, PETSCII.
```

Architecture — `architecture-castle.png`:

```text
Use case: stylized-concept
Asset type: versioned PETSCII conversion benchmark reference, architecture category
Primary request: A compact medieval hill castle with two towers, one gate, crenellated walls, and a stone bridge.
Scene/backdrop: simple dusk sky and a single dark hill.
Style/medium: clean flat-color retro editorial illustration, not pixel art and not PETSCII.
Composition/framing: 8:5 landscape canvas; castle centered and fills the middle; strong symmetrical architectural silhouette; readable at 40x25 cells.
Lighting/mood: quiet dusk with a few warm window lights.
Color palette: 7 to 9 solid colors, violet-blue sky, cool stone, warm small accents.
Constraints: crisp straight and stepped contours; continuous rooflines and wall edges; large coherent color regions.
Avoid: text, watermark, frame, gradients, noise, masonry microtexture, dithering, halftone, photorealism, pixel art, ASCII art, PETSCII.
```

Action — `action-dragon.png`:

```text
Use case: stylized-concept
Asset type: versioned PETSCII conversion benchmark reference, action category
Primary request: A winged dragon in a sweeping S-shaped flying pose, breathing one bold arc of flame.
Scene/backdrop: simple dark red sunset sky above one black mountain ridge.
Style/medium: clean flat-color retro editorial illustration, not pixel art and not PETSCII.
Composition/framing: 8:5 landscape canvas; dragon and flame form one readable action silhouette across the frame; generous margin; readable at 40x25 cells.
Lighting/mood: dramatic and energetic.
Color palette: 7 to 9 solid colors, dark silhouette, red-orange flame, restrained highlights.
Constraints: crisp continuous wing, neck, tail, and flame contours; large coherent color regions; no isolated speckles.
Avoid: text, watermark, frame, gradients, noise, scales, tiny anatomy, dithering, halftone, photorealism, pixel art, ASCII art, PETSCII.
```

Measure it without touching the versioned benchmark using `--dither-eval`, which
converts each reference three ways (baseline top-1, current pipeline, and the
pipeline with dithering) and reports reconstruction and perceptual tone for
each. Add `--corpus-prior <report.json>` to also score how human-like each
result's glyph layout is, using a `petii corpus` report as a bigram prior:

```sh
cargo run -p petii --release -- corpus apps/petview/assets --output tmp/petview-prior.json
cargo run -p petii --release -- benchmark \
  tools/petii/benchmark/v1/prompts.json \
  --reference-dir tools/petii/benchmark/v1/references \
  --output-dir tmp/petii-dither-eval --width 60 --height 38 \
  --dither-eval --corpus-prior tmp/petview-prior.json
```

Dithering improves perceptual tone on gradient-heavy scenes (a dusk-sky castle
fell 27%) while slightly raising the corpus bigram cost, so the corpus prior
doubles as a guard against over-dithering. Human preference remains the product
gate; both scores are diagnostic.

When a corpus prior is supplied, the dither arm is regularized: each proposed
dither cell is kept only when its perceived-tone gain outweighs the corpus cost
of texturing it beside its neighbors, so marginal dithering at the fringe of a
region reverts to solid while the high-contrast core survives. The trade weight
is `PETII_DITHER_LAMBDA` (default 1.0); higher values favor corpus layout
fidelity over tone accuracy. `convert_image_dithered_prior` exposes the same
regularized path programmatically.

The versioned v1 suite includes five recorded reference-generator outputs and
an `expected-report.json` snapshot. CI compares structure exactly while allowing
only `1e-12` relative/absolute floating-point noise. Reproduce it locally with:

```bash
cargo test -p petii --release \
  benchmark::tests::recorded_benchmark_v1_matches_snapshot \
  -- --ignored --exact
```

If an intentional algorithm change alters the report, inspect all five rendered
pairs before replacing the snapshot. A changed reference image requires a new
versioned suite rather than an in-place overwrite.

`benchmark/v1/human-evaluation.json` records the first blinded side-by-side A/B
review, including the hidden assignment, preference per case, and comparison
with the deterministic score winner. Human preference remains the product gate;
the scalar score is diagnostic and does not override it.

`benchmark/v1/human-evaluation-v2.json` records the second blinded review after
adding strong-reference structure protection. Castle changed from a baseline
preference to a candidate preference, but airship moved in the opposite
direction and dragon still preferred the baseline; the human gate remains 60%.

`benchmark/v1/human-evaluation-v3.json` records a stricter structure gate where
strong-edge glyph edits must repair an actual cross-cell break. At 40x25 the
human gate remains 60%; dragon is the only case that preferred the baseline in
all three rounds. The product's width-60 target must be evaluated separately.

## Current limitations

- This milestone targets static PETSCII scenes, not animation or game code.
- AI repair is restricted to validated regions, palette roles, density,
  contrast, silhouette protection, and individual cells. Crop repair is
  validated but deferred because it requires rebuilding the candidate pool.
- Deterministic scores measure reconstruction and PETSCII structure; they do
  not replace blinded human evaluation of artistic quality.
