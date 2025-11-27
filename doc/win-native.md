# Windows Native Installation Guide

RustPixel fully supports native Windows development! This guide covers installation and setup for Windows 10/11 without WSL.

## 🎯 Supported Rendering Modes

| Mode | Status | Requirements | Performance |
|------|--------|--------------|-------------|
| **WGPU** | ✅ Recommended | None | Excellent |
| **Winit-OpenGL** | ✅ Supported | None | Good |
| **Terminal** | ✅ Supported | Nerd Font (optional) | Good |
| **SDL2** | ⚠️ Extra Setup | SDL2 libraries | Good |

## 🚀 Quick Start (Recommended)

### Step 1: Install Rust

**Option A: Using Rustup (Recommended)**
1. Visit [https://rustup.rs/](https://rustup.rs/)
2. Download and run `rustup-init.exe`
3. Follow the installation instructions

**Option B: Using Windows Package Manager**
```powershell
winget install Rustlang.Rustup
```

**Option C: Using Chocolatey**
```powershell
choco install rust
```

### Step 2: Install RustPixel

```powershell
# Install cargo-pixel tool
cargo install rust_pixel

# First run will automatically clone the repository
cargo pixel

# Navigate to the workspace
cd %USERPROFILE%\rust_pixel_work
```

### Step 3: Test Installation

```powershell
# Test WGPU mode (recommended for best performance)
cargo pixel r petview wgpu

# Test Glow-OpenGL mode
cargo pixel r petview glow

# Test Terminal mode
cargo pixel r petview term
```

## 🎨 Font Setup for Terminal Mode

For the best terminal experience, install a Nerd Font:

### Option A: Manual Installation
1. Download [DroidSansMono Nerd Font](https://github.com/ryanoasis/nerd-fonts/releases)
2. Extract and install the `.ttf` files
3. Configure your terminal to use the font

### Option B: Using Scoop
```powershell
scoop bucket add nerd-fonts
scoop install DroidSansMono-NF
```

### Terminal Configuration
- **Windows Terminal**: Settings → Profiles → Defaults → Font face → "DroidSansMono Nerd Font"
- **PowerShell**: Properties → Font → "DroidSansMono Nerd Font"
- **Command Prompt**: Properties → Font → "DroidSansMono Nerd Font"

## 🔧 SDL2 Setup (Optional)

SDL2 mode requires additional setup but offers cross-platform compatibility:

### Step 1: Download SDL2 Development Libraries
1. Visit [SDL2 Releases](https://github.com/libsdl-org/SDL/releases)
2. Download `SDL2-devel-x.x.x-VC.zip` (Visual C++ version)
3. Extract to `C:\SDL2\` (recommended location)

### Step 2: Set Environment Variables

**Option A: Using System Properties**
1. Open "Environment Variables" from System Properties
2. Add system variable: `SDL2_DIR = C:\SDL2`
3. Add to PATH: `C:\SDL2\lib\x64`

**Option B: Using PowerShell**
```powershell
# Set for current session
$env:SDL2_DIR = "C:\SDL2"
$env:PATH += ";C:\SDL2\lib\x64"

# Set permanently (requires restart)
[Environment]::SetEnvironmentVariable("SDL2_DIR", "C:\SDL2", "Machine")
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "Machine")
[Environment]::SetEnvironmentVariable("PATH", $currentPath + ";C:\SDL2\lib\x64", "Machine")
```

### Step 3: Test SDL2 Mode
```powershell
cargo pixel r petview sdl
```

## 🎮 Building and Running Games

### Available Demo Games
```powershell
# Terminal versions
cargo pixel r tetris term
cargo pixel r snake term
cargo pixel r tower term
cargo pixel r poker term

# Graphics versions (WGPU recommended)
cargo pixel r tetris wgpu
cargo pixel r snake wgpu
cargo pixel r tower wgpu
cargo pixel r poker wgpu

# SDL2 versions (if configured)
cargo pixel r tetris sdl
cargo pixel r snake sdl
```

### Creating New Projects
```powershell
# Create a new game project
cargo pixel creat games mygame
cd mygame

# Run your game
cargo pixel r mygame term    # Terminal mode
cargo pixel r mygame wgpu    # WGPU mode
```

## 🛠️ Development Tools

### Asset Management
```powershell
# Convert sprites
cargo pixel asset ./input_sprites ./output

# Edit pixel art
cargo pixel edit wgpu . myfile.pix

# Convert PETSCII images
cargo pixel petii image.png 40 25

# Extract symbols from images
cargo pixel symbol image.png 8

# Sequence frame player
cargo pixel ssf wgpu . animation.ssf
```

### Build for Different Targets
```powershell
# Build for Windows
cargo build --release

# Build for Web (requires wasm-pack)
cargo pixel build mygame web

# Build with specific features
cargo build --features "wgpu"
cargo build --features "glow"
cargo build --features "sdl"
```

## 🔍 Troubleshooting

### Common Issues

**1. "cargo-pixel not found"**
```powershell
# Ensure Cargo bin directory is in PATH
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
```

**2. WGPU/Graphics Issues**
- Update GPU drivers
- Ensure DirectX 12 or Vulkan support
- Try Winit-OpenGL mode as fallback

**3. SDL2 Linking Errors**
- Verify SDL2_DIR environment variable
- Check SDL2 DLL is in PATH
- Use x64 version for 64-bit systems

**4. Font Rendering Issues in Terminal**
- Install Nerd Font properly
- Restart terminal after font installation
- Check terminal font settings

### Performance Tips
- Use WGPU mode for best performance
- Ensure GPU drivers are up to date
- Close unnecessary applications when running graphics mode
- Use Release builds for better performance: `cargo pixel r game wgpu -r`

## 📋 Requirements Summary

### Minimum Requirements
- Windows 10 or later
- Rust 1.71+
- 4GB RAM
- GPU with DirectX 11+ support

### Recommended Setup
- Windows 11
- 8GB+ RAM
- Dedicated GPU with DirectX 12/Vulkan support
- SSD storage
- Nerd Font installed

## 🆘 Getting Help

- **Documentation**: Check `doc/` folder for additional guides
- **Issues**: [GitHub Issues](https://github.com/zipxing/rust_pixel/issues)
- **Examples**: Explore `apps/` folder for game examples
- **Tools**: Check `tools/` folder for utility documentation

## 🔄 Updating RustPixel

```powershell
# Update cargo-pixel tool
cargo install rust_pixel --force

# Update workspace (when in rust_pixel directory)
git pull origin main

# Rebuild tools
cargo install --path . --force
```

Happy coding with RustPixel on Windows! 🎮✨ 