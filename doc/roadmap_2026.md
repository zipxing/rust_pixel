# RustPixel 2026 Roadmap

## Vision

Build RustPixel into a **Tile-first retro engine**: one codebase supporting **Games + Hybrid TUI** (Terminal / Native Window / Web), with **AI** as an integral part of the character art asset pipeline.

---

## Priority Levels

### P0 - Must Do (Differentiation & Promotion)

These define RustPixel's unique value proposition.

#### 1. Hybrid TUI Flagship Showcase: TUI Presentation
- Same UI runs consistently in native window and terminal (prove with GIF)
- Slide DSL + layout/panel + transitions + input (page navigation)
- Demonstrates "Write Once, Run Anywhere" for TUI apps

#### 2. Game Showcase Full Scripting: Tetris → Tower
- Rust handles engine core only
- Gameplay and rules driven by BASIC scripts
- Unified demo template: `init` / `update` / `draw` / `on_key`

#### 3. AI PETSCII Asset Capability (Search First, Generate Later)
- 2000+ PETSCII artworks + tags → natural language retrieval (RAG/embedding)
- Direct insertion into presentations/games
- Unified pluggable `AssetProvider` interface for future generation upgrades

---

### P1 - High Priority (Engineering "Tile-first")

Solidify the core architecture.

#### 1. Stabilize Core Rendering Abstraction
- Cell/Buffer/Sprite/Panel composition rules
- Clipping, z-order, color model, font metrics consistency

#### 2. Cross-Platform Consistency & Backend Standardization
- Converge Terminal / Native Window input and rendering differences
- Provide consistent key names and event model

#### 3. Asset Pipeline & Pack Specification
- Charset / Palette / PETSCII Pack: import/export, versioning, caching
- Hot reload for better development experience

---

### P2 - Medium Priority (Toolchain & Ecosystem)

Extend long-term competitiveness.

#### 1. Upgrade Palette Tool to Hybrid TUI
- Tool-type showcase
- Part of character art pipeline
- Enhance palette/pack specification and preview capabilities

#### 2. Petview Scripting & Content Browsing
- BASIC handles UI / shortcuts / flow
- Rust handles IO / decoding / rendering

#### 3. AI Generation Capability Upgrade (Actual Image Generation)
- After search capability is stable
- Text → PETSCII token grid generation
- Controllable: size / colors / charset / seed

---

### P3 - Lower Priority (Nice to Have)

Avoid slowing down main development.

#### 1. News AI TUI App
- Better as a "pluggable provider" example
- Avoid external dependencies making demos fragile

#### 2. Web/WASM Showcase
- Run Presentation or a small game first
- Serves as proof of "Run Anywhere"

---

## Summary

**Optimal Path**: First establish the three selling points with **Presentation + BASIC Games + AI Asset Search** (Hybrid TUI + Scripting + AI Character Art Pipeline). Then engineer and ecosystem-ize with **asset specifications, cross-platform consistency, and plugin architecture**.

---

## Three Pillars

| Pillar | Description |
|--------|-------------|
| **Hybrid TUI** | Same code runs in Terminal, Native Window, and Web |
| **Scripting** | BASIC drives game logic, Rust handles engine |
| **AI Art Pipeline** | PETSCII asset search → generation |
