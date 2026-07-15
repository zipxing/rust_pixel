## Context

`tools/petii` currently resizes an input image to the target character grid, extracts each 8×8 grayscale block, optionally binarizes it, selects one glyph using a 10-dimensional structural feature distance, and assigns foreground/background colors. The selection is local to each cell. This is fast and deterministic but does not optimize whole-image readability or semantic alignment.

The engine already represents PETSCII art in the desired final form: fixed glyph identifiers, foreground/background colors, `.pix` composition, and the unified symbol/tile renderer. This change operates before asset loading and does not introduce arbitrary per-image tiles.

## Goals / Non-Goals

### Goals

- Improve static PETSCII composition while preserving a locked character set and palette.
- Accept a natural-language prompt as the generation intent.
- Use AI for semantic planning, reference generation, critique, and bounded repair guidance.
- Keep glyph selection and `.pix` validity under deterministic Rust control.
- Produce inspectable artifacts and quantitative comparisons for every iteration.
- Work without GPU rendering by using a CPU preview renderer.

### Non-Goals

- Training or fine-tuning a PETSCII foundation model.
- Generating arbitrary RGBA tiles or extending the selected character set per image.
- Sprite-sheet animation, `.ssf` generation, game code, or complete cartridge generation.
- Guaranteeing one-shot quality from a single model response.
- Replacing the existing deterministic `petii` workflow.

## Decisions

### Decision: Use a hybrid optimizer/critic loop

The model SHALL NOT emit the complete cell grid as unconstrained text. Rust code owns candidate construction, aspect-preserving grid dimensions, and validation. AI responses are limited to an `ArtPlan`, candidate scores, regional critiques, and repair directives validated against a JSON schema.

Alternatives considered:

- Direct LLM grid generation: rejected for poor spatial reliability and difficult validation.
- End-to-end model training: deferred because the existing corpus is not sufficiently paired with captions and structured annotations.
- Image generation followed by one-pass conversion: retained only as the baseline seed because it creates a filtered-image appearance without global correction.

### Decision: Separate reference generation from PETSCII synthesis

For prompt-only runs, an image provider produces a deliberately low-detail, high-contrast reference image. That image is temporary guidance and never becomes a custom runtime Tile. The final artifact is synthesized exclusively from the selected PETSCII glyph set.

An input-image mode remains available so optimizer development and tests do not require an image-generation provider.

### Decision: Generate a candidate pool before AI critique

The initial pass produces multiple candidates by varying a bounded configuration set:

- crop and subject scale;
- contrast and edge emphasis;
- background estimation;
- character subset and density penalty;
- palette reduction and foreground/background assignment;
- raw-pixel, structural-feature, edge, and multi-scale loss weights.

Each cell retains its top-K glyph/color alternatives. A deterministic whole-image optimizer selects among these alternatives using reconstruction, edge continuity, neighborhood coherence, palette, density, and protected-region losses.

### Decision: Critique rendered previews, not internal cells alone

Candidates are rendered to PNG using the exact PETSCII glyph bitmap and palette. The multimodal critic receives the prompt, reference, and candidate previews and returns:

- semantic fidelity, subject readability, composition, palette coherence, contour continuity, and PETSCII authenticity scores;
- normalized bounding boxes for problematic regions;
- bounded repair operations such as `increase_contrast`, `simplify_region`, `protect_silhouette`, `reduce_density`, `shift_crop`, `change_palette_role`, and limited `replace_cell` edits;
- a concise explanation stored for diagnostics.

All fields are schema-validated and clamped before use.

### Decision: Best-so-far is monotonic

Every iteration keeps the highest-scoring valid candidate from all prior iterations. Invalid repairs, lower-scoring candidates, timeouts, or provider errors cannot replace the best-so-far artifact.

### Decision: Record provider responses for replay

Each run directory stores a manifest containing model identifiers, seeds when supported, request hashes, response payloads with secrets removed, optimizer configuration, candidate scores, and artifact paths. Replaying from recorded provider responses must reproduce deterministic optimizer output.

## Proposed Pipeline

```text
prompt or input image
        |
        v
ArtPlan + constrained reference image
        |
        v
baseline conversion + parameter sweep
        |
        v
top-K per-cell candidates
        |
        v
whole-image deterministic optimization
        |
        v
CPU render candidate previews
        |
        v
multimodal critic JSON
        |
        v
bounded parameter/region/cell repairs
        |
        +---- repeat within iteration/cost budget
        |
        v
best `.pix` + PNG + manifest + candidate gallery
```

## Proposed Internal Components

```text
tools/petii/src/
├── candidate.rs       top-K glyph/color candidate generation
├── optimizer.rs       global and regional objective optimization
├── preview.rs         CPU `.pix` to PNG renderer
├── ai/
│   ├── provider.rs    provider-neutral traits and HTTP configuration
│   ├── schema.rs      ArtPlan, Critique, RepairDirective
│   └── loop.rs        bounded orchestration and best-so-far tracking
├── run_artifacts.rs   manifest and replay data
└── benchmark.rs       offline metrics and comparison runner
```

The final module layout may remain flatter if implementation stays small.

## Scoring

The deterministic score is a weighted combination of:

- multi-scale luminance reconstruction;
- edge-map reconstruction;
- cross-cell contour continuity;
- palette coherence and contrast;
- glyph-density regularization;
- protected-region silhouette quality;
- optional critic scores.

AI scores are advisory rather than the only acceptance criterion. Candidate validity and baseline retention are deterministic gates.

## Safety, Cost, and Privacy

- Default maximum iterations: 4.
- Default candidates per iteration: 4.
- Provider calls have explicit connect/read timeouts and limited retries.
- Input image dimensions and response sizes are bounded before decoding.
- API keys are read from environment/config and never written into manifests or logs.
- The tool executes no model-produced code and accepts only schema-validated directives.
- An offline mode supports baseline conversion, preview rendering, optimization, and replay from recorded responses.

## Benchmark Plan

Create an initial versioned set of prompts covering:

- single objects and portraits;
- foreground subject with simple background;
- landscape and architecture;
- action composition;
- light/dark and limited-palette scenes.

For each prompt, retain the baseline, loop result, run manifest, deterministic metrics, and blinded human preference result. The benchmark is a product-quality gate, not a model-training dataset.

## Risks / Trade-offs

- A stronger reference image may still convert poorly. Mitigation: prompt the reference provider for flat composition, strong silhouette, low detail, and target aspect ratio.
- VLM scores may be inconsistent. Mitigation: schema validation, deterministic metrics, best-so-far retention, and human A/B evaluation.
- Global optimization may be slow. Mitigation: top-K pruning, regional repair, small character grids (40 columns by default), and hard iteration limits.
- AI cost may exceed product value. Mitigation: candidate pooling before critique, configurable providers, cached responses, and per-run cost budgets.
- Optimizing pixel similarity can preserve the filtered-image look. Mitigation: explicit density, continuity, silhouette, and PETSCII-authenticity objectives.

## Migration Plan

1. Refactor current conversion into reusable library functions without changing CLI output.
2. Add candidate generation, CPU preview, and deterministic global optimization in offline mode.
3. Add recorded-response replay and schemas.
4. Add live provider integration behind an explicit AI command/flag.
5. Add benchmark gates before exposing the capability to game generation.

Rollback consists of disabling the AI command/flag; the existing `petii` conversion remains intact.

## Open Questions

- Whether the first live provider adapter should target one concrete API or an OpenAI-compatible subset.
- Whether the launch palette should remain ANSI-256 or use a stricter C64/RustPixel 16-color palette.
- Whether human selection among the top four candidates is required for the first product-facing version.
