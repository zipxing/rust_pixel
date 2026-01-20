# basic-scripting Specification

## Purpose
TBD - created by archiving change add-basic-scripting. Update Purpose after archive.
## Requirements
### Requirement: BASIC Script Loading
The system SHALL provide a mechanism to load and parse BASIC source code into an executable program.

#### Scenario: Load BASIC program from string
- **WHEN** a valid BASIC program string is provided to GameBridge::load_program()
- **THEN** the program is parsed and stored in the runtime ready for execution

#### Scenario: Load BASIC program from file
- **WHEN** a .bas file path is provided
- **THEN** the file content is read and loaded as a BASIC program

---

### Requirement: Coroutine Execution
The system SHALL support coroutine-style execution allowing BASIC scripts to pause and resume across game frames.

#### Scenario: WAIT statement pauses execution
- **WHEN** a WAIT 0.5 statement is executed
- **THEN** the execution state changes to Waiting with resume time set to current_time + 0.5 seconds
- **AND** control returns to the game loop

#### Scenario: YIELD statement yields to next frame
- **WHEN** a YIELD statement is executed
- **THEN** the execution state changes to Yielded
- **AND** execution resumes on the next call to step()

#### Scenario: WAITKEY statement waits for input
- **WHEN** a WAITKEY statement is executed
- **THEN** the execution state changes to WaitingFor(KeyPress)
- **AND** execution resumes when any key is pressed

#### Scenario: Coroutine resumes after wait time
- **WHEN** the game loop calls step() after the resume_at time has passed
- **THEN** execution continues from the statement after WAIT

---

### Requirement: Graphics Drawing Functions
The system SHALL provide BASIC functions to draw graphics primitives on the game panel.

#### Scenario: PLOT draws a character
- **WHEN** PLOT 10, 5, "@", 2, 0 is executed
- **THEN** the character "@" is drawn at position (10, 5) with foreground color 2 and background color 0

#### Scenario: CLS clears the screen
- **WHEN** CLS is executed
- **THEN** all cells in the panel are reset to empty

#### Scenario: LINE draws a line
- **WHEN** LINE 0, 0, 10, 10, "*" is executed
- **THEN** a line of "*" characters is drawn from (0,0) to (10,10)

#### Scenario: BOX draws a rectangle
- **WHEN** BOX 5, 5, 10, 8, 1 is executed
- **THEN** a rectangle border is drawn at position (5,5) with width 10, height 8, using border style 1

---

### Requirement: Sprite Management
The system SHALL provide BASIC functions to create, manipulate, and query sprites.

#### Scenario: SPRITE creates or updates a sprite
- **WHEN** SPRITE 1, 10, 20, "@" is executed
- **THEN** sprite with ID 1 is created/updated at position (10, 20) with symbol "@"

#### Scenario: SMOVE moves a sprite relatively
- **WHEN** SMOVE 1, 2, -1 is executed
- **THEN** sprite 1's position changes by (+2, -1) from its current position

#### Scenario: SHIDE controls sprite visibility
- **WHEN** SHIDE 1, 1 is executed
- **THEN** sprite 1 becomes hidden and is not rendered

#### Scenario: SPRITEX returns sprite X position
- **WHEN** X = SPRITEX(1) is executed
- **THEN** variable X contains the X coordinate of sprite 1

#### Scenario: SPRITEHIT detects collision
- **WHEN** H = SPRITEHIT(1, 2) is executed
- **THEN** H is 1 if sprites 1 and 2 overlap, otherwise 0

---

### Requirement: Input Handling
The system SHALL provide BASIC functions to query keyboard and mouse input.

#### Scenario: INKEY returns last pressed key
- **WHEN** K = INKEY() is executed
- **THEN** K contains the ASCII code of the last pressed key, or 0 if no key was pressed

#### Scenario: KEY checks if specific key is held
- **WHEN** IF KEY("W") THEN Y=Y-1 is executed
- **THEN** the condition is true if the W key is currently held down

#### Scenario: MOUSEX returns mouse X position
- **WHEN** MX = MOUSEX() is executed
- **THEN** MX contains the current X coordinate of the mouse cursor

#### Scenario: MOUSEB returns mouse button state
- **WHEN** MB = MOUSEB() is executed
- **THEN** MB contains a bitmask of currently pressed mouse buttons (1=left, 2=right, 4=middle)

---

### Requirement: Game Lifecycle Hooks
The system SHALL call specific BASIC subroutines at defined points in the game lifecycle.

#### Scenario: ON_INIT called at startup
- **WHEN** the game starts
- **THEN** GOSUB 1000 (ON_INIT) is called once to initialize game state

#### Scenario: ON_TICK called each frame
- **WHEN** the game loop updates
- **THEN** GOSUB 2000 (ON_TICK) is called with DT variable set to frame delta time

#### Scenario: ON_DRAW called for rendering
- **WHEN** the game loop renders
- **THEN** GOSUB 3000 (ON_DRAW) is called to perform rendering operations

---

### Requirement: GameBridge Integration
The system SHALL provide a GameBridge struct that connects the BASIC executor with rust_pixel's rendering system.

#### Scenario: GameBridge syncs sprites to Panel
- **WHEN** GameBridge::draw() is called
- **THEN** all BASIC-created sprites are rendered to the rust_pixel Panel

#### Scenario: GameBridge converts input events
- **WHEN** rust_pixel Event::Key is received
- **THEN** the key state is updated in the BASIC input context for INKEY/KEY functions

#### Scenario: GameBridge manages game time
- **WHEN** GameBridge::update() is called with dt parameter
- **THEN** the accumulated game time is updated and coroutine resume times are checked

