## ADDED Requirements

### Requirement: TUI Symbol Texture Support

The rendering system SHALL support loading and using a separate TUI symbol texture (`symbols_tui.png`) with 8x16 pixel cells for terminal-style character rendering in graphics mode.

#### Scenario: TUI texture loading in graphics mode
- **WHEN** the application enables TUI mode in graphics mode
- **THEN** the system loads `symbols_tui.png` with 128x128 symbols at 8x16 pixels each
- **AND** the TUI texture is used for rendering Main Buffer content
- **AND** the standard `symbols.png` (16x16) is used for Pixel Sprites

#### Scenario: Text mode remains unchanged
- **WHEN** the application runs in text mode
- **THEN** no TUI texture is loaded
- **AND** all rendering uses terminal character cells as before

### Requirement: Dual Coordinate System for Mouse Events

The input system SHALL provide two coordinate systems in mouse events: TUI coordinates for thin characters (1:2 aspect ratio) and Sprite coordinates for square characters (1:1 aspect ratio).

#### Scenario: Mouse event with dual coordinates
- **WHEN** a mouse event occurs in graphics mode with TUI enabled
- **THEN** the `MouseEvent` contains `column_tui` and `row_tui` calculated using 8-pixel width
- **AND** the `MouseEvent` contains `column` and `row` calculated using 16-pixel width
- **AND** both coordinate pairs are independently accurate for their respective rendering layers

#### Scenario: Backward compatibility for existing code
- **WHEN** existing code accesses `MouseEvent.column` and `MouseEvent.row`
- **THEN** the values are calculated using the standard Sprite coordinate system (16x16)
- **AND** no changes are required to existing mouse handling code

### Requirement: TUI Layer Rendering Priority

The rendering system SHALL ensure that the TUI layer (Main Buffer) is always rendered on top of all Pixel Sprite layers in graphics mode.

#### Scenario: TUI overlay on game sprites
- **WHEN** the scene contains both Pixel Sprites and TUI elements
- **THEN** all Pixel Sprites are rendered first
- **AND** the TUI layer (Main Buffer) is rendered last
- **AND** TUI elements appear on top of all game objects

#### Scenario: Rendering order in RenderCell array
- **WHEN** generating the RenderCell array
- **THEN** Pixel Sprite cells are added first
- **AND** Main Buffer (TUI) cells are added last
- **AND** the GPU renders in array order, ensuring correct layering

### Requirement: TUI Symbol Dimensions Configuration

The system SHALL provide separate global configuration for TUI and Sprite symbol dimensions.

#### Scenario: TUI dimensions initialization
- **WHEN** TUI mode is enabled
- **THEN** `PIXEL_TUI_WIDTH` is set to 8.0 pixels
- **AND** `PIXEL_TUI_HEIGHT` is set to 16.0 pixels
- **AND** these values are used for Main Buffer rendering

#### Scenario: Sprite dimensions remain unchanged
- **WHEN** rendering Pixel Sprites
- **THEN** `PIXEL_SYM_WIDTH` remains 16.0 pixels
- **AND** `PIXEL_SYM_HEIGHT` remains 16.0 pixels
- **AND** existing sprite rendering is unaffected

### Requirement: Single Draw Call Performance

The rendering system SHALL maintain single draw call performance by merging TUI and Sprite render cells into a unified RenderCell array.

#### Scenario: Unified rendering pipeline
- **WHEN** rendering a frame with both TUI and Sprites
- **THEN** all RenderCells (TUI and Sprite) are in a single array
- **AND** the GPU processes all cells in one instanced draw call
- **AND** rendering performance is equivalent to the current system

#### Scenario: Variable cell dimensions in shader
- **WHEN** the shader processes RenderCells with different dimensions
- **THEN** each cell's `w` and `h` fields correctly specify its size
- **AND** TUI cells (8x16) and Sprite cells (16x16) render correctly in the same pass

### Requirement: TUI Architecture Always Enabled

The system SHALL always enable TUI architecture in graphics mode, supporting mixed rendering of TUI (Main Buffer) and game sprites (Pixel Sprites) without requiring configuration.

#### Scenario: TUI architecture initialized on startup
- **WHEN** the application starts in graphics mode
- **THEN** both TUI and Sprite symbol textures are loaded
- **AND** both `PIXEL_TUI_*` and `PIXEL_SYM_*` dimensions are initialized
- **AND** mouse events include both TUI and Sprite coordinates
- **AND** the rendering pipeline supports mixed TUI and Sprite rendering

#### Scenario: Application chooses rendering approach
- **WHEN** an application uses only Pixel Sprites (no Main Buffer content)
- **THEN** TUI layer renders as empty (no overhead)
- **AND** the application works exactly as before
- **WHEN** an application uses Main Buffer for TUI elements
- **THEN** TUI elements render with 8x16 thin characters
- **AND** TUI layer appears on top of all Pixel Sprites

### Requirement: UI Component TUI Coordinate Support

UI components SHALL use TUI coordinates (`column_tui`, `row_tui`) for mouse event handling when rendering in the Main Buffer.

#### Scenario: UI component mouse hit testing
- **WHEN** a UI component (e.g., Button) receives a mouse event
- **THEN** it uses `mouse_event.column_tui` and `mouse_event.row_tui` for hit testing
- **AND** the hit test correctly identifies clicks on TUI-rendered components
- **AND** the component responds to user interaction accurately

#### Scenario: Game sprite mouse handling unchanged
- **WHEN** game code handles mouse events for Pixel Sprites
- **THEN** it continues to use `mouse_event.column` and `mouse_event.row`
- **AND** sprite interaction remains accurate and unchanged

### Requirement: TUI Style Modifier Support

The TUI rendering system SHALL support text style modifiers (bold, italic, underlined, dim, reversed, crossed-out, hidden) in graphics mode, providing visual parity with text mode styling capabilities.

#### Scenario: RenderCell modifier field support
- **WHEN** converting Cell to RenderCell for TUI rendering
- **THEN** the `RenderCell` includes a `modifier` field containing the Cell's modifier bitflags
- **AND** the modifier information is preserved through the rendering pipeline
- **AND** the GPU shader receives modifier data for each character

#### Scenario: Bold text rendering
- **WHEN** TUI content uses `Style::default().add_modifier(Modifier::BOLD)`
- **THEN** the text appears with increased visual weight in graphics mode
- **AND** the bold effect is achieved through color intensity adjustment in the rendering pipeline
- **AND** the RGB values are multiplied by 1.3 (clamped to 1.0) before creating RenderCell
- **AND** the styling provides clear visual distinction from normal text

#### Scenario: Italic text rendering
- **WHEN** TUI content uses `Style::default().add_modifier(Modifier::ITALIC)`
- **THEN** the text appears with italic slant in graphics mode
- **AND** the italic effect is achieved through vertex transformation in the shader
- **AND** the slant angle provides clear visual distinction from normal text

#### Scenario: Underlined text rendering
- **WHEN** TUI content uses `Style::default().add_modifier(Modifier::UNDERLINED)`
- **THEN** the text appears with an underline in graphics mode
- **AND** the underline is rendered as a horizontal line below the character
- **AND** the underline color matches the foreground color

#### Scenario: Dim text rendering
- **WHEN** TUI content uses `Style::default().add_modifier(Modifier::DIM)`
- **THEN** the text appears with reduced opacity in graphics mode
- **AND** the dim effect is achieved through alpha channel adjustment in the rendering pipeline
- **AND** the alpha value is multiplied by 0.6 before creating RenderCell
- **AND** the text remains readable but visually de-emphasized

#### Scenario: Reversed text rendering
- **WHEN** TUI content uses `Style::default().add_modifier(Modifier::REVERSED)`
- **THEN** the foreground and background colors are swapped in graphics mode
- **AND** the color swap is handled in the rendering pipeline before creating RenderCell
- **AND** the original foreground color becomes the background color
- **AND** the original background color becomes the foreground color
- **AND** the visual effect matches terminal reverse video

#### Scenario: Crossed-out text rendering
- **WHEN** TUI content uses `Style::default().add_modifier(Modifier::CROSSED_OUT)`
- **THEN** the text appears with a horizontal line through the middle in graphics mode
- **AND** the strikethrough line is rendered in the fragment shader
- **AND** the line color matches the foreground color

#### Scenario: Hidden text rendering
- **WHEN** TUI content uses `Style::default().add_modifier(Modifier::HIDDEN)`
- **THEN** the text is completely transparent in graphics mode
- **AND** the hidden effect is achieved by setting alpha to 0.0 in the rendering pipeline
- **AND** the character space is preserved but content is invisible

#### Scenario: Multiple modifier combination
- **WHEN** TUI content combines multiple modifiers (e.g., BOLD + ITALIC + UNDERLINED)
- **THEN** all specified effects are applied simultaneously in graphics mode
- **AND** the combined effects do not interfere with each other
- **AND** the visual result matches the expected terminal appearance

#### Scenario: Text mode compatibility maintained
- **WHEN** the application runs in text mode
- **THEN** all modifier effects continue to use crossterm ANSI sequences
- **AND** no changes are made to existing text mode styling behavior
- **AND** the visual appearance remains identical to current implementation

#### Scenario: Blink modifiers ignored
- **WHEN** TUI content uses `Modifier::SLOW_BLINK` or `Modifier::RAPID_BLINK`
- **THEN** the blink modifiers are ignored in graphics mode
- **AND** the text renders as normal without blinking animation
- **AND** no error or warning is generated for unsupported blink effects

