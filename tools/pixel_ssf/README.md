# PixelSSF - SSF Sequence Frame Player

A command-line tool for playing RustPixel's SSF (Sequence Frame) animation files.

## Features

- **Multi-mode Support**: Works in both terminal mode and graphics mode (SDL/Winit/WGPU)
- **Interactive Controls**: Full playback control with keyboard shortcuts
- **Multiple Playback Options**: Auto-play, manual frame stepping, loop modes
- **Speed Control**: Adjustable playback speed from 0.5x to 20x
- **Cross-platform**: Runs on Windows, macOS, and Linux

## Usage

```bash
# Play default animation file
cargo run --bin pixel_ssf

# Play specific SSF file
cargo run --bin pixel_ssf -- path/to/animation.ssf

# Build for different modes
cargo run --bin pixel_ssf --features sdl    # SDL graphics mode
cargo run --bin pixel_ssf --features winit  # Winit graphics mode  
cargo run --bin pixel_ssf --features wgpu   # WGPU graphics mode
cargo run --bin pixel_ssf --features term   # Terminal mode (default)
```

## Controls

| Key | Action |
|-----|--------|
| `Space` | Toggle auto play/pause |
| `←` / `→` | Previous/Next frame |
| `R` | Reset to first frame |
| `L` | Toggle loop mode |
| `+` / `=` | Increase speed |
| `-` | Decrease speed |
| `Q` | Quit |

## Default Files

The tool comes with several example SSF files:
- `assets/sdq/dance.ssf` - Dancing animation (default)
- `assets/sdq/fire.ssf` - Fire effect
- `assets/sdq/heart.gif` - Heart animation

## Building

```bash
# Terminal mode (minimal dependencies)
cargo build --features term

# Graphics modes
cargo build --features sdl
cargo build --features winit  
cargo build --features wgpu
```

## SSF File Format

SSF (Sequence Frame) files are compressed animation files used by RustPixel. They support:
- Multiple frame formats (ASCII, Unicode, PETSCII)
- Frame-by-frame compression
- Texture mapping for graphics modes
- Variable frame rates

For more information about the SSF format, see the RustPixel documentation. 