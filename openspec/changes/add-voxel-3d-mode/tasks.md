## 1. Specification

- [ ] 1.1 Confirm capability boundaries for independent `3d` mode, voxel renderer, and shared runtime reuse
- [ ] 1.2 Confirm the asset mapping contract `cube face -> PUA -> symbol -> tile`
- [ ] 1.3 Confirm PUA-A block allocation strategy for voxel face materials

## 2. Runtime Entry

- [ ] 2.1 Add CLI mode selection for `cargo pixel r <app> 3d`
- [ ] 2.2 Define the runtime contract for 3D applications without breaking existing `term` / `g` / `w`
- [ ] 2.3 Reuse event/timer/context/game loop plumbing for 3D mode
- [ ] 2.4 Decide the exact integration points in `src/game.rs`, `src/context.rs`, and `src/render/adapter.rs`

## 3. Voxel Rendering MVP

- [ ] 3.1 Add voxel world primitives (`Block`, `Chunk`, `ChunkMesh`, `Camera`)
- [ ] 3.2 Add visible-face mesh generation for block worlds
- [ ] 3.3 Add WGPU rendering path for voxel mesh, depth testing, and camera matrices
- [ ] 3.4 Add Minecraft-style MVP demo with sufficient display fidelity
- [ ] 3.5 Scaffold `src/render/voxel/{mod,world,mesh,camera,material,atlas,renderer}.rs`
- [ ] 3.6 Scaffold `apps/voxel_demo/` as the validation app

## 4. Asset Pipeline

- [ ] 4.1 Define voxel material schema supporting `all`, `top/bottom/side`, and six-face mappings
- [ ] 4.2 Define how PUA aliases and human-readable symbols are generated and loaded
- [ ] 4.3 Ensure runtime can resolve face materials to atlas tiles without per-frame string lookup
- [ ] 4.4 Define whether the single source of truth lives in `symbol_map.json` extensions or `voxel_materials.json`
- [ ] 4.5 Define voxel PUA block allocation and validation rules

## 5. Validation

- [ ] 5.1 Smoke-test the new `3d` mode with a demo app
- [ ] 5.2 Verify existing `term` / `g` / `w` modes remain unaffected
- [ ] 5.3 Document any unresolved design follow-ups before implementation approval
