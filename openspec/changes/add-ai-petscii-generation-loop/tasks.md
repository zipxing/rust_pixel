## 1. Baseline Refactor and Tests

- [x] 1.1 Move current `petii` conversion logic into reusable library modules while preserving existing CLI behavior.
- [x] 1.2 Define typed PETSCII cell-grid, character-set, palette, and conversion configuration structures.
- [x] 1.3 Add golden tests for current conversion output and `.pix` validity.
- [x] 1.4 Add a versioned benchmark prompt/reference set and baseline artifact format.

## 2. Deterministic Quality Pipeline

- [x] 2.1 Return top-K glyph candidates and distances per cell instead of only the nearest glyph.
- [ ] 2.2 Add configurable preprocessing, crop, contrast, edge, density, and palette variants.
- [x] 2.3 Implement a CPU renderer from the typed PETSCII grid to PNG using the exact glyph bitmap and palette.
- [x] 2.4 Implement whole-image scoring for reconstruction, edges, contour continuity, palette coherence, and glyph density.
- [x] 2.5 Implement bounded global/regional optimization over the top-K candidate space.
- [x] 2.6 Verify deterministic output for identical input, configuration, and seed.
- [x] 2.7 Derive AI output height from the reference-image aspect ratio unless explicitly overridden.

## 3. AI Schemas and Replay

- [x] 3.1 Define and validate `ArtPlan`, `Critique`, region, score, and `RepairDirective` schemas.
- [ ] 3.2 Implement run directories and redacted manifests containing inputs, configurations, candidates, scores, and responses.
- [ ] 3.3 Implement offline replay from recorded provider responses.
- [x] 3.4 Add malformed, oversized, out-of-bounds, and unsupported directive tests.

## 4. Provider Integration and Loop

- [x] 4.1 Define provider-neutral reference-generator and multimodal-critic traits.
- [x] 4.2 Implement one initial HTTP provider adapter with timeouts, retry limits, response-size bounds, and secret redaction.
- [x] 4.3 Implement prompt-to-reference generation using low-detail, high-contrast PETSCII-oriented art direction.
- [x] 4.4 Implement candidate preview submission and structured critique parsing.
- [ ] 4.5 Translate validated repair directives into bounded optimizer configuration and regional/cell mutations.
- [ ] 4.6 Implement iteration, candidate, time, and cost budgets with monotonic best-so-far retention.
- [ ] 4.7 Implement deterministic fallback when providers fail or return invalid data.

## 5. CLI and User Artifacts

- [x] 5.1 Add an explicit experimental AI generation command/flag without changing existing `cargo pixel petii` behavior.
- [ ] 5.2 Support prompt-only, input-image, offline, and replay modes.
- [x] 5.3 Emit final `.pix`, rendered PNG, candidate gallery, critique summary, and run manifest.
- [x] 5.4 Document configuration, API key handling, budgets, limitations, and reproducibility.

## 6. Evaluation

- [ ] 6.1 Implement a benchmark runner comparing the baseline and loop result.
- [ ] 6.2 Run the benchmark with recorded provider responses for repeatable CI checks.
- [ ] 6.3 Conduct a blinded human A/B evaluation and record preferences.
- [ ] 6.4 Confirm at least 70% of loop outputs win or tie the baseline before declaring the MVP complete.
