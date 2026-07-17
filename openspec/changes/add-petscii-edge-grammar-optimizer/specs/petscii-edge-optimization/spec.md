## ADDED Requirements

### Requirement: Bitmap-Derived PETSCII Edge Topology

The system SHALL describe every allowed PETSCII glyph and inverse form using deterministic topology derived from its rendered bitmap.

#### Scenario: Topology catalog generation
- **WHEN** the optimizer initializes a configured PETSCII character set
- **THEN** it derives each glyph's boundary ports, tangent directions, connected components, endpoints, junctions, fill side, density, centroid, and role
- **AND** the catalog uses the exact bitmap used by the preview and runtime renderer

#### Scenario: Ambiguous glyph classification
- **WHEN** bitmap geometry is insufficient to assign an intended role
- **THEN** the system MAY apply a documented reviewed override
- **AND** the override is character-role based rather than scene, object, coordinate, or input-image specific

### Requirement: Reference Contour Graph

The system SHALL convert meaningful cleaned reference edges into stable cross-cell contour chains before optimizing glyph selection.

#### Scenario: Strong contour crosses cells
- **WHEN** a cleaned strong edge enters and exits one or more grid cells
- **THEN** the system records ordered target ports, local tangent, curvature, fill side, confidence, and junction metadata for the affected cells
- **AND** assigns stable contour and traversal order identifiers

#### Scenario: Weak isolated edge component
- **WHEN** an edge component is below the configured strength, size, or connectivity threshold
- **THEN** it is excluded from the contour graph
- **AND** does not force glyph edits

#### Scenario: Legitimate open contour
- **WHEN** a contour reaches an image boundary, low-confidence occlusion, or other geometry-supported endpoint
- **THEN** the endpoint can be marked legal
- **AND** is not penalized as an unexplained dangling stroke

### Requirement: Baseline-Preserving Candidate-Lattice Optimization

The system SHALL jointly optimize the existing per-cell Top-K PETSCII candidate lattice and local losses without removing the Top-1 baseline as a valid final result.

#### Scenario: Strong contour cell
- **WHEN** a cell belongs to the configured strong contour band
- **THEN** the system retains the baseline and bounded Top-K candidates
- **AND** MAY add a bounded set of allowed topology-compatible graphic glyphs
- **AND** preserves validated foreground/background color roles

#### Scenario: Flat or unrelated cell
- **WHEN** a cell is flat or outside the configured contour band
- **THEN** the system leaves the baseline cell unchanged by default

#### Scenario: No acceptable optimized candidate
- **WHEN** all optimized candidates violate validity or quality gates
- **THEN** the unchanged baseline remains the final result

### Requirement: Cross-Cell PETSCII Edge Grammar

The system SHALL select contour glyph sequences using rules for port connection, direction, curvature, fill-side consistency, endpoints, spurs, and junctions in addition to per-cell reference similarity.

#### Scenario: Continuous contour
- **WHEN** a reference contour crosses a shared cell boundary
- **THEN** selected neighboring glyphs expose compatible ports within the configured positional tolerance
- **AND** their tangent directions and fill sides remain consistent with the reference contour

#### Scenario: Direction change
- **WHEN** the reference contour bends across cells
- **THEN** the selected glyph sequence uses compatible corner, diagonal, wedge, or transition roles
- **AND** avoids an unsupported abrupt tangent or fill-side flip

#### Scenario: Unsupported branch or spur
- **WHEN** a candidate sequence creates a short branch, repeated backtrack, false junction, or non-legal dangling endpoint absent from the reference contour
- **THEN** the system penalizes or rejects that sequence according to bounded rules

#### Scenario: Valid junction
- **WHEN** the reference contour graph contains a supported junction
- **THEN** the optimizer coordinates all incident contour ports
- **AND** does not optimize each incident chain independently into incompatible glyphs

### Requirement: Deterministic Bounded Optimization

The system SHALL optimize edge grammar within explicit candidate, edit, iteration, and time budgets using stable ordering and tie-breaking.

#### Scenario: Identical input and configuration
- **WHEN** the optimizer runs repeatedly with identical input image, grid, character set, palette, and configuration
- **THEN** it produces byte-for-byte identical `.pix` output and metrics

#### Scenario: Optimization budget exhausted
- **WHEN** any configured candidate, edit, iteration, chain, or time budget is exhausted
- **THEN** the system stops further search
- **AND** returns the best valid result found so far or the unchanged baseline

### Requirement: Edge Quality Gate and Fallback

The system SHALL compare optimized output with its own baseline using edge continuity and reference fidelity metrics before accepting it.

#### Scenario: Optimization improves edge grammar
- **WHEN** optimized output reduces the configured contour-break, unexpected-endpoint, spur, or false-junction objective
- **AND** its reference reconstruction loss remains within the allowed regression threshold
- **THEN** the optimized result can replace the baseline

#### Scenario: Optimization damages reference fidelity
- **WHEN** optimized output exceeds the configured reference-loss regression threshold
- **THEN** the affected region or whole result falls back to the baseline according to configuration

### Requirement: Explainable Edge Diagnostics

The system SHALL expose deterministic metrics and optional visual diagnostics for edge grammar decisions.

#### Scenario: Diagnostic artifact generation
- **WHEN** diagnostics are enabled for a conversion
- **THEN** the run records baseline and optimized contour-break rate, unexpected-endpoint rate, spur count, false-junction count, contour coverage, reference loss, and edit ratio
- **AND** emits overlays identifying reference contours, target ports, selected glyph ports, breaks, spurs, and junctions

### Requirement: Human-Art-Calibrated General Rules

The system SHALL use aggregate human PETSCII corpus analysis to calibrate and evaluate general edge rules without introducing runtime scene templates.

#### Scenario: Corpus calibration
- **WHEN** maintainers analyze the versioned `petview` fixture selection or full local corpus
- **THEN** the analyzer reports glyph roles, edge-port adjacency, endpoints, junctions, spurs, and direction-transition statistics reproducibly
- **AND** the aggregate results can inform default weights and topology corrections

#### Scenario: Runtime optimization
- **WHEN** the optimizer processes a new image
- **THEN** it does not select rules by artwork ID, scene name, object label, or copied neighborhood template
- **AND** applies the same bounded geometry rules to all inputs

### Requirement: Edge Optimization Benchmark

The system SHALL compare the edge grammar optimizer against the unchanged per-cell baseline on a versioned multi-subject benchmark.

#### Scenario: Automated benchmark run
- **WHEN** the benchmark processes all reference cases
- **THEN** it saves baseline and optimized `.pix`, PNG, metrics, runtime, configuration, and diagnostic overlays
- **AND** reports aggregate and per-case changes

#### Scenario: Quality acceptance
- **WHEN** the capability is considered for default enablement
- **THEN** median strong-contour break rate and unexpected-endpoint rate have each decreased by at least 30 percent
- **AND** no accepted result exceeds 5 percent reference-loss regression
- **AND** at least 70 percent of blinded human comparisons prefer or tie the optimized output
