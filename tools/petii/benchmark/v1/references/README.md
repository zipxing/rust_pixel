# PETII Benchmark v1 References

These five fixed references cover portrait, object, scene, architecture, and
action compositions. They are deliberately low-detail raster illustrations,
not PETSCII artwork, so the benchmark measures image-to-PETSCII conversion
rather than recovering an already quantized grid.

The images were generated on 2026-07-17 with Codex's built-in image generation
tool. Every prompt requested an 8:5 landscape composition readable at 40x25,
large coherent color regions, crisp continuous contours, no text or watermark,
and no pixel-art, ASCII-art, or PETSCII treatment. Subject-specific prompts were:

- `portrait-witch.png`: a three-quarter moon witch portrait with a broad pointed
  hat, dark indigo background, and a pale crescent moon.
- `object-airship.png`: a side-view retro airship with a large oval balloon,
  compact gondola, clear blue sky, and broad cloud bands.
- `scene-forest.png`: layered pine forest, distant hills, pale morning sun, and a
  continuous S-shaped stream.
- `architecture-castle.png`: a centered two-tower medieval castle, crenellated
  walls, central gate, bridge, and violet-blue dusk sky.
- `action-dragon.png`: a dark flying dragon silhouette breathing a broad arc of
  orange flame above a mountain ridge at red sunset.

SHA-256 checksums:

```text
ac3e70c46edf4f66a3edd663825c9fbf7b9383155ced9f3b90e83dab3f5d1507  action-dragon.png
08e187eb5db3a7f531c0557930ce0219aa7808f29e85a4e6f72b03014ce6e246  architecture-castle.png
e34ac9e846fec9f5d37f6df3c579048bf474d10c6ab62cb918f55994370a2c5b  object-airship.png
6f8f12fca425f0a78f8e40cdba1673d5bdb7340cdba5e9231a3d427bdbcffec0  portrait-witch.png
364136e3bea72efe7ee9859e815746f6e910c3a208c3bf72e18d529c1d753083  scene-forest.png
```

Do not regenerate or overwrite these files in place. A changed image belongs in
a new versioned benchmark directory so historical reports remain reproducible.
`../expected-report.json` is the deterministic baseline produced from these
files with `BenchmarkOptions::default()`. CI requires identical structure and
allows only `1e-12` relative/absolute floating-point noise.
