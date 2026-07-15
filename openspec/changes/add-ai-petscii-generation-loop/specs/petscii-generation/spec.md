## ADDED Requirements

### Requirement: Fixed-Charset PETSCII Output

The system SHALL generate artwork exclusively from glyphs in the configured PETSCII character set and colors in the configured palette.

#### Scenario: Valid AI-assisted output
- **WHEN** an AI-assisted generation run completes successfully
- **THEN** every output cell references an allowed PETSCII glyph
- **AND** every foreground and background color belongs to the configured palette
- **AND** the result is saved as a valid `.pix` artifact

#### Scenario: Invalid model directive
- **WHEN** a model directive requests an unavailable glyph, color, or out-of-bounds cell
- **THEN** the directive is rejected or clamped according to the validated schema
- **AND** no arbitrary custom Tile is added to the output

### Requirement: Natural-Language Art Intent

The system SHALL accept a natural-language prompt describing the intended static PETSCII artwork.

#### Scenario: Prompt-only generation
- **WHEN** a user supplies a prompt without an input image and a reference-image provider is configured
- **THEN** the system creates an art plan and constrained reference image
- **AND** uses them only as guidance for fixed-character-set PETSCII synthesis

#### Scenario: Input-image generation
- **WHEN** a user supplies a prompt and an input image
- **THEN** the system uses the input image as reference without requiring image generation
- **AND** still emits only fixed-character-set PETSCII cells

### Requirement: Candidate Generation and Preview

The system SHALL generate multiple bounded PETSCII candidates and render each candidate to a preview image using the configured glyph bitmaps and palette.

#### Scenario: Initial candidate pool
- **WHEN** conversion begins
- **THEN** the system includes the existing nearest-match output as a baseline candidate
- **AND** generates additional candidates from bounded preprocessing, palette, glyph-density, crop, and scoring configurations

#### Scenario: Headless preview
- **WHEN** a PETSCII candidate is evaluated
- **THEN** the system renders it to PNG without requiring a terminal, desktop window, or WGPU device

### Requirement: Aspect-Preserving Grid Dimensions

The AI-assisted workflow SHALL preserve the reference-image aspect ratio by default instead of forcing a 40×25 grid.

#### Scenario: Square generated reference
- **WHEN** the generated reference image has a 1:1 aspect ratio
- **AND** the user keeps the default width of 40 without specifying a height
- **THEN** the PETSCII output grid is 40×40

#### Scenario: Non-square input reference
- **WHEN** the user supplies a reference image without specifying a height
- **THEN** the system derives the row count from the configured width and reference-image aspect ratio
- **AND** rounds to the nearest non-zero whole row

#### Scenario: Explicit grid dimensions
- **WHEN** the user explicitly supplies both width and height
- **THEN** those dimensions override automatic aspect-ratio derivation

### Requirement: Structured AI Critique

The system SHALL accept only schema-validated structured critique and repair directives from the multimodal critic.

#### Scenario: Valid critique
- **WHEN** the critic evaluates candidate previews
- **THEN** it returns bounded scores, normalized problem regions, and supported repair directives
- **AND** the response is validated before it can affect optimization

#### Scenario: Malformed critique
- **WHEN** the critic returns malformed JSON, unsupported operations, oversized content, or invalid coordinates
- **THEN** the system rejects the invalid fields
- **AND** continues with the best valid candidate or deterministic fallback

### Requirement: Bounded Iterative Repair

The system SHALL iteratively improve candidates within explicit limits on iterations, candidate count, time, provider calls, and cost.

#### Scenario: Repair iteration
- **WHEN** a valid critique identifies a low-quality region
- **THEN** the system translates the critique into bounded configuration, regional, or cell-level mutations
- **AND** renders and scores the repaired candidates before considering them for selection

#### Scenario: Budget exhausted
- **WHEN** any configured generation budget is exhausted
- **THEN** the system stops requesting or evaluating new candidates
- **AND** returns the highest-scoring valid candidate produced so far

### Requirement: Monotonic Best Candidate

The system SHALL retain the best valid candidate across all iterations and SHALL NOT replace it with a lower-scoring or invalid candidate.

#### Scenario: Repair degrades quality
- **WHEN** a repaired candidate scores below the current best candidate
- **THEN** the repaired candidate is retained only as a diagnostic artifact
- **AND** the current best candidate remains selected

#### Scenario: All AI operations fail
- **WHEN** every AI request fails or returns invalid data
- **THEN** the initial deterministic baseline remains available as the final output

### Requirement: Reproducible Run Artifacts

The system SHALL save sufficient redacted inputs, configurations, outputs, and provider responses to inspect and replay a generation run.

#### Scenario: Successful run manifest
- **WHEN** a generation run completes
- **THEN** its directory contains the prompt, reference metadata, seeds, conversion configurations, candidate previews, scores, critiques, final `.pix`, and final PNG
- **AND** no API key or authorization header is stored

#### Scenario: Offline replay
- **WHEN** a user replays a run using its recorded provider responses
- **THEN** deterministic conversion and optimization stages reproduce the same selected PETSCII grid

### Requirement: Baseline Comparison

The system SHALL compare AI-loop output against the existing deterministic `petii` result on a versioned benchmark.

#### Scenario: Automated benchmark
- **WHEN** the benchmark runner processes a case
- **THEN** it saves both baseline and loop artifacts with their deterministic metrics

#### Scenario: MVP quality gate
- **WHEN** the MVP is evaluated using blinded human A/B comparisons
- **THEN** at least 70 percent of loop outputs win or tie their baseline counterparts before the capability is declared complete

### Requirement: Backward-Compatible Deterministic Conversion

The system SHALL preserve the existing non-AI image-to-PETSCII workflow.

#### Scenario: Existing command usage
- **WHEN** a user invokes the current `petii` conversion without AI options
- **THEN** the tool produces the same output format and does not require network access or AI credentials
