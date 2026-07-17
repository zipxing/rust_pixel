## 1. Corpus Analysis and Benchmark Baseline

- [ ] 1.1 Add a reproducible analyzer for all `apps/petview/assets` glyph, color-role, edge-port, adjacency, endpoint, junction, and spur statistics.
- [ ] 1.2 Select and document a small versioned set of human PETSCII structural fixtures without duplicating the full gallery.
- [ ] 1.3 Add ordinary-image benchmark references covering horizontal, vertical, diagonal, curved, closed, occluded, portrait, landscape, and architecture contours.
- [ ] 1.4 Record current mode 2 baseline `.pix`, PNG, runtime, reconstruction, continuity, endpoint, junction, and spur metrics.

## 2. Glyph Edge Topology

- [ ] 2.1 Define typed glyph roles, edge ports, tangent directions, fill-side, component, endpoint, junction, density, and centroid descriptors.
- [ ] 2.2 Derive topology descriptors deterministically from every allowed glyph bitmap and inverse form.
- [ ] 2.3 Add a reviewed override mechanism for geometrically ambiguous glyph roles with documented rationale.
- [ ] 2.4 Add exhaustive glyph topology tests and a diagnostic atlas showing bitmap, ports, directions, fill side, and role.

## 3. Reference Contour Graph

- [ ] 3.1 Trace the cleaned edge map into ordered contour chains with stable IDs and scan order.
- [ ] 3.2 Map each contour through cells into target entry/exit ports, tangent, curvature, fill side, confidence, and junction metadata.
- [ ] 3.3 Detect legal image-border, weak-confidence, and occlusion-like endpoints without object-specific rules.
- [ ] 3.4 Add synthetic contour fixtures for straight lines, diagonals, corners, loops, T-junctions, crossings, occlusions, and borders.

## 4. Edge Grammar Optimizer

- [x] 4.1 Treat the per-cell Top-K lattice and local losses as optimizer input, preserve Top-1 as candidate zero/fallback, and freeze flat/non-contour cells by default.
- [x] 4.2 Expand strong-contour candidates from bounded Top-K and topology-compatible graphic glyphs while preserving valid color roles.
- [ ] 4.3 Implement the reference, port, tangent, fill-side, endpoint, spur, junction, curvature, and edit penalty terms.
- [x] 4.4 Implement deterministic chain-level optimization for non-junction contours with stable tie-breaking.
- [x] 4.5 Implement fixed-budget junction/conflict coordination and local repair.
- [x] 4.6 Add per-region or whole-image quality gating and automatic fallback to the unchanged baseline.
- [ ] 4.7 Expose bounded configuration for candidate count, port tolerance, weights, iteration count, edit budget, and enable/disable behavior.

## 5. Metrics and Diagnostics

- [x] 5.1 Implement strong-contour break rate, unexpected endpoint rate, short-spur count, false-junction count, contour coverage, reference loss, and edit ratio metrics.
- [x] 5.2 Emit baseline/optimized metrics and deterministic debug overlays for reference contours, target ports, selected glyph ports, breaks, spurs, and junctions.
- [x] 5.3 Add tests proving metrics distinguish connected, broken, dangling, over-connected, and visually regressed fixtures.
- [x] 5.4 Verify output glyph/palette validity and byte-for-byte determinism for identical inputs and configurations.

## 6. Evaluation and Rollout

- [ ] 6.1 Run the versioned benchmark and tune defaults using aggregate results rather than individual scene exceptions.
- [ ] 6.2 Confirm median strong-contour break rate and unexpected endpoint rate each improve by at least 30% over baseline.
- [x] 6.3 Confirm reference reconstruction loss does not regress by more than 5% for any accepted optimized result.
- [ ] 6.4 Record runtime and enforce explicit candidate, iteration, edit, and total-time budgets.
- [ ] 6.5 Conduct blinded human A/B evaluation and confirm at least 70% of optimized outputs win or tie baseline.
- [ ] 6.6 Document the optimizer, metrics, debug artifacts, configuration, limitations, fallback, and reproduction commands.
- [ ] 6.7 Enable the optimizer by default for the agreed mode 2 workflow only after all quality gates pass.
