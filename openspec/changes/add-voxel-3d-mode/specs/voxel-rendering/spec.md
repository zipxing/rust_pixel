## ADDED Requirements

### Requirement: Independent 3D Mode
The system SHALL provide an independent `3d` runtime mode for voxel applications without changing the default behavior of existing `term`, `g`, or `w` modes.

#### Scenario: Run an application in 3D mode
- **WHEN** the user runs `cargo pixel r <app> 3d`
- **THEN** the engine starts the application using the 3D voxel rendering pipeline
- **AND** the existing 2D/TUI mode selection semantics remain unchanged

#### Scenario: Existing modes remain compatible
- **WHEN** an existing application is run with `term`, `g`, or `w`
- **THEN** it continues to use the pre-existing runtime and rendering contract
- **AND** it does not require 3D-specific code paths

### Requirement: Shared Runtime Reuse
The system SHALL reuse existing runtime modules for events, timers, context, and game-loop orchestration in 3D mode wherever those modules are backend-agnostic.

#### Scenario: Timer and event systems work in 3D mode
- **WHEN** a 3D application registers timers and receives input events
- **THEN** it uses the same event and timer infrastructure as other rust_pixel modes
- **AND** the 3D mode does not require a separate timer or event subsystem

#### Scenario: 3D rendering data remains separate from 2D cells
- **WHEN** a 3D application renders voxel content
- **THEN** the runtime uses dedicated voxel world, camera, and mesh structures
- **AND** it does not require encoding voxel geometry through `Scene/Layer/Sprite/Buffer/Cell`

### Requirement: Minecraft-Style Voxel Rendering
The system SHALL support Minecraft-style block voxel rendering with enough visual precision for recognizable cube worlds and face-based texturing.

#### Scenario: Render visible cube faces
- **WHEN** a voxel chunk contains solid blocks adjacent to empty space
- **THEN** the renderer generates and draws only the visible faces of those blocks
- **AND** hidden internal faces are omitted from the draw mesh

#### Scenario: Render a navigable voxel scene
- **WHEN** a 3D application creates a camera and a voxel world
- **THEN** the engine renders a depth-tested scene from the camera viewpoint
- **AND** the output quality is sufficient for Minecraft-style presentation and interaction

### Requirement: Face Material Mapping Through PUA
The system SHALL support voxel face materials through the mapping chain `cube face -> PUA -> symbol -> tile`.

#### Scenario: Resolve a cube face material
- **WHEN** a voxel material defines a face using a PUA code
- **THEN** the runtime resolves that PUA to its symbol alias and atlas tile metadata
- **AND** the face is rendered using the resolved tile

#### Scenario: Use human-readable aliases in tools or configs
- **WHEN** tooling, editors, or configuration files refer to a voxel face by symbol alias
- **THEN** the engine can map that alias back to the corresponding PUA/material entry
- **AND** the alias remains a readable representation rather than the sole runtime render key

### Requirement: Unified PUA-A Material Encoding
The system SHALL use Supplementary PUA-A as the unified encoding domain for voxel face materials that participate in sprite-style atlas lookup.

#### Scenario: Reserve voxel material blocks within sprite-style PUA space
- **WHEN** voxel face materials are assigned encoded identifiers
- **THEN** they occupy explicitly allocated PUA-A sprite blocks
- **AND** those allocations do not reuse TUI, emoji, or CJK logical regions as voxel face IDs

#### Scenario: Cache tile resolution outside the hot path
- **WHEN** voxel materials are loaded into runtime structures
- **THEN** the engine resolves and caches the tile references needed for rendering
- **AND** rendering visible chunk faces does not require per-face string lookup every frame

### Requirement: Flexible Cube Face Material Definitions
The system SHALL support block materials defined as a single texture, `top/bottom/side`, or six independent face textures.

#### Scenario: Define a uniform material
- **WHEN** a block material specifies `all`
- **THEN** all six faces of that block use the same resolved face material

#### Scenario: Define a grass-like material
- **WHEN** a block material specifies `top`, `bottom`, and `side`
- **THEN** the renderer applies those mappings to the appropriate cube faces

#### Scenario: Define a directional material
- **WHEN** a block material specifies six independent faces
- **THEN** each face of the cube uses its own resolved face material
