## Why

The current `petii` tool converts each image block independently to its nearest PETSCII glyph. It produces valid `.pix` files, but often loses global composition, connected contours, subject readability, and palette coherence. RustPixel needs a measurable first step toward natural-language-generated character art before relying on that capability for AI-generated games.

## What Changes

- Add an experimental AI-assisted PETSCII generation loop for static artwork whose grid preserves the reference-image aspect ratio (default width 40).
- Preserve the existing PETSCII constraint: every output cell is a glyph from the selected fixed character set plus foreground/background palette indices.
- Generate and retain multiple deterministic `petii` candidates instead of accepting a single per-cell nearest match.
- Add a CPU preview renderer so candidates can be evaluated without launching a terminal or WGPU window.
- Add provider-neutral interfaces for reference-image generation and multimodal critique.
- Require the critic to return structured scores, regions, and bounded repair directives; the model does not directly replace the full cell grid.
- Iterate by adjusting conversion parameters, regional weights, glyph density, palette, and a bounded set of cell edits while always retaining the best prior candidate.
- Save the source prompt, reference image, configurations, rendered candidates, critiques, scores, seeds, and final `.pix` in a reproducible run directory.
- Provide a benchmark and human A/B evaluation against the existing `petii` baseline.
- Keep the existing non-AI image-to-PETSCII command working unchanged.

## Impact

- Affected specs: new `petscii-generation` capability.
- Affected code: `tools/petii`, `src/render/symbols.rs`, and `tools/cargo-pixel` command wiring.
- New optional external dependency: an image-generation provider and a vision-language critic accessed through configurable HTTP APIs.
- No rendering-engine API break and no change to the fixed-character-set Tile architecture.

## Success Criteria

1. Every completed run emits a valid `.pix` using only the configured PETSCII glyphs and palette.
2. The loop never returns a candidate scored below its own initial baseline candidate.
3. At least 70% of outputs win or tie the existing `petii` baseline in a blinded human A/B test over the initial benchmark prompt set.
4. Failed or unavailable AI calls fall back to the deterministic baseline and preserve diagnostic artifacts.
5. Cost, iteration count, candidate count, timeouts, and output dimensions are explicitly bounded.
