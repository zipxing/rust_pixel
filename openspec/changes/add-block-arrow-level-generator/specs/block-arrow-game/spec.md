## ADDED Requirements

### Requirement: Block Shape System
The system SHALL define polyomino shapes (monomino through tetromino) as relative coordinate sets, and SHALL generate all unique rotation/flip variants for each shape.

#### Scenario: Shape variant generation
- **WHEN** the shape library is initialized
- **THEN** all shapes have correct variant counts (e.g., T-tetromino has 4 variants, O-tetromino has 1)

#### Scenario: Shape rotation
- **WHEN** a shape with cells [(0,0),(1,0),(2,0),(1,1)] is rotated 90 degrees
- **THEN** the resulting cells are normalized to start from (0,0)

### Requirement: Pixel Art Coverage Algorithm
The system SHALL accept a colored bitmap (up to 16x16 grid of u8 values where 0=background, 1-15=colors) and SHALL produce a non-overlapping set of polyomino placements that covers all non-zero cells exactly once. Each placed block SHALL only cover cells of the same color, so that the assembled blocks reproduce the original bitmap colors exactly.

#### Scenario: Full coverage of colored bitmap
- **WHEN** a valid colored bitmap is provided (e.g., 9x9 pixel art with color values 1-15)
- **THEN** the algorithm returns a set of placed blocks covering all non-zero pixels
- **THEN** no two blocks overlap on any cell

#### Scenario: Color consistency
- **WHEN** a block is placed on the board
- **THEN** all cells covered by that block MUST have the same color value in the original bitmap
- **THEN** the block's color attribute matches that bitmap color value

#### Scenario: Per-color-region coverage
- **WHEN** the bitmap contains multiple colors (e.g., color 1 and color 3)
- **THEN** each color region is covered independently by its own set of blocks
- **THEN** no single block spans cells of different colors

#### Scenario: Prefer larger blocks
- **WHEN** coverage is computed for a color region
- **THEN** tetrominos are preferred over triominos, which are preferred over dominos and monominos

### Requirement: Arrow Assignment with Solvability
The system SHALL assign exactly one direction arrow (Up/Down/Left/Right) to each placed block, such that there exists a removal order where every block can fly away in its arrow direction without obstruction.

#### Scenario: Solvable level generation
- **WHEN** a coverage is successfully computed
- **THEN** an arrow assignment is found such that all blocks can be removed sequentially

#### Scenario: Fly-away check
- **WHEN** a block has arrow direction "Right"
- **THEN** the block can fly away only if no other remaining block occupies any cell to the right of the block's rightmost cells

### Requirement: Interactive Terminal Gameplay
The system SHALL render the puzzle board in terminal mode, allow cursor movement with arrow keys, and allow the player to trigger block removal with Space/Enter.

#### Scenario: Block removal
- **WHEN** the player selects a block and presses Space
- **THEN** if the block can fly away in its arrow direction (no obstruction by remaining blocks), it is removed from the board

#### Scenario: Win condition
- **WHEN** all blocks have been removed from the board
- **THEN** the game displays a victory message

#### Scenario: Invalid move
- **WHEN** the player tries to fly a block that is obstructed
- **THEN** the block remains on the board and a feedback indication is shown
