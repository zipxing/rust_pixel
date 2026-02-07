## ADDED Requirements

### Requirement: Markdown Slide Parsing
The system SHALL parse standard Markdown files into a sequence of slides using `comrak`, splitting on horizontal rules (`---`) or `<!-- end_slide -->` as slide boundaries.

#### Scenario: Parse simple markdown into slides
- **WHEN** a Markdown file containing two `---` separators is provided
- **THEN** three SlideContent objects are produced, each containing their respective elements

#### Scenario: Parse end_slide comment as slide boundary
- **WHEN** a Markdown file uses `<!-- end_slide -->` comments as delimiters
- **THEN** slides are split at each `<!-- end_slide -->` boundary, equivalent to `---`

#### Scenario: Parse front matter configuration
- **WHEN** a Markdown file begins with YAML front matter (`---` delimited)
- **THEN** the configuration (theme, transition, title_animation, code_theme, margin) is extracted and applied to the presentation

#### Scenario: Parse headings as slide titles
- **WHEN** a slide contains a `# Heading` or `## Heading` element
- **THEN** the heading is represented as a SlideElement::Title with the corresponding level and text

#### Scenario: Parse code blocks with language info
- **WHEN** a slide contains a fenced code block with language identifier (e.g., ` ```rust `)
- **THEN** the code block is represented as a SlideElement::CodeBlock with language and source code preserved

#### Scenario: Parse code block with line numbers flag
- **WHEN** a code block uses ` ```rust +line_numbers ` syntax
- **THEN** the CodeBlock element has `show_line_numbers: true` and renders with line number prefixes

#### Scenario: Parse lists
- **WHEN** a slide contains an unordered or ordered list
- **THEN** the list is represented as a SlideElement::List with items and list type preserved

#### Scenario: Parse tables
- **WHEN** a slide contains a Markdown table
- **THEN** the table is represented as a SlideElement::Table with headers, rows, and column alignment preserved

---

### Requirement: Slide Navigation
The system SHALL support keyboard-based navigation between slides with visual feedback of current position.

#### Scenario: Navigate to next step or slide
- **WHEN** the user presses the Right arrow key or Space
- **THEN** if the current slide has remaining pause steps, the next step is revealed; otherwise the presentation advances to the next slide with a transition effect

#### Scenario: Navigate to previous slide
- **WHEN** the user presses the Left arrow key or Backspace
- **THEN** the presentation goes back to the previous slide with a transition effect

#### Scenario: Jump to first or last slide
- **WHEN** the user presses Home or End
- **THEN** the presentation jumps to the first or last slide respectively

#### Scenario: Exit presentation
- **WHEN** the user presses 'q' or Escape
- **THEN** the presentation exits and the terminal is restored to its original state

#### Scenario: Display slide position
- **WHEN** any slide is displayed
- **THEN** a status bar at the bottom shows `[current / total]` slide indicator

---

### Requirement: Code Syntax Highlighting
The system SHALL render code blocks with syntax highlighting using `syntect`, mapping highlighted colors to RustPixel Color::Rgba.

#### Scenario: Highlight Rust code block
- **WHEN** a code block with language `rust` is rendered
- **THEN** keywords, strings, comments, and other tokens are displayed in distinct colors matching the selected theme

#### Scenario: Select highlight theme via front matter
- **WHEN** front matter contains `code_theme: base16-ocean`
- **THEN** the specified syntect theme is used for all code block highlighting

#### Scenario: Fallback for unknown language
- **WHEN** a code block specifies an unrecognized language
- **THEN** the code is rendered as plain text without highlighting

---

### Requirement: Slide Transitions
The system SHALL apply visual transition effects when switching between slides, supporting both CPU-based BufferTransition and GPU-based GpuTransition.

#### Scenario: Apply BufferTransition in terminal mode
- **WHEN** a slide transition occurs in terminal mode
- **THEN** a BufferTransition effect (e.g., Dissolve, WipeLeft) animates from the old slide to the new slide over a configurable duration

#### Scenario: Apply GpuTransition in graphics mode
- **WHEN** a slide transition occurs in graphics mode and a GPU transition is configured
- **THEN** a GpuTransition effect (e.g., Ripple, Heart, RotateZoom) is applied via blend_rts

#### Scenario: Configure transition type via front matter
- **WHEN** front matter contains `transition: dissolve`
- **THEN** the specified transition type is used for all slide changes

#### Scenario: Default transition when not configured
- **WHEN** no transition is specified in front matter
- **THEN** the Dissolve transition is used as the default

---

### Requirement: Text Animations
The system SHALL support per-slide text animation effects for title elements, using RustPixel Label widget animations.

#### Scenario: Typewriter animation on slide title
- **WHEN** a slide is displayed with `title_animation: typewriter` configured
- **THEN** the title text appears character by character with a blinking cursor effect

#### Scenario: FadeIn animation on slide title
- **WHEN** a slide is displayed with `title_animation: fade_in` configured
- **THEN** the title characters scale up from invisible to full size left-to-right

#### Scenario: No animation when disabled
- **WHEN** `title_animation: none` is configured
- **THEN** the title text appears immediately without animation

#### Scenario: Animation resets on slide change
- **WHEN** the user navigates to a new slide
- **THEN** the title animation restarts from the beginning

---

### Requirement: Image and Animation Rendering
The system SHALL support embedding .pix images and .ssf animations in slides via standard Markdown image syntax, loading them through RustPixel's AssetManager pipeline.

#### Scenario: Render .pix image in slide
- **WHEN** a slide contains `![Logo](assets/logo.pix)`
- **THEN** the .pix file is loaded via `asset2sprite!` and rendered as a Sprite at the image position in the slide

#### Scenario: Render .ssf animation in slide
- **WHEN** a slide contains `![Dance](assets/dance.ssf)`
- **THEN** the .ssf file is loaded and its frames are played back automatically, advancing frame_idx each tick

#### Scenario: Fallback for unsupported image formats
- **WHEN** a slide contains `![Photo](photo.png)` with a non-.pix/.ssf extension
- **THEN** a placeholder text `[Image: Photo]` is displayed at the image position

#### Scenario: ImageProvider trait extensibility
- **WHEN** a custom ImageProvider implementation is registered
- **THEN** the provider's `load_image()` method is called to convert the image path into a renderable Buffer content

---

### Requirement: WASM/Web Deployment
The system SHALL support compilation to WebAssembly for running presentations in a web browser, using RustPixel's WebAdapter and WebGL2 rendering.

#### Scenario: Build WASM module
- **WHEN** `wasm-pack build` is run in the `wasm/` directory
- **THEN** a valid WebAssembly module is produced with JavaScript bindings

#### Scenario: Run presentation in browser
- **WHEN** the WASM module is loaded in a browser with the JavaScript bridge (index.js)
- **THEN** the presentation renders on an HTML canvas with WebGL2, supporting keyboard navigation and slide transitions

#### Scenario: Async asset loading in WASM
- **WHEN** a slide references a .pix or .ssf file in web mode
- **THEN** the asset is loaded asynchronously via `js_load_asset()` → `fetch()` → `on_asset_loaded()`, and rendered once loaded

#### Scenario: Code highlighting works in WASM
- **WHEN** a code block is rendered in web mode
- **THEN** syntax highlighting via syntect functions correctly (pure Rust, no FFI dependency)

---

### Requirement: Incremental Display (Pause)
The system SHALL support `<!-- pause -->` comment commands to enable step-by-step content reveal within a single slide.

#### Scenario: Pause splits slide into steps
- **WHEN** a slide contains `<!-- pause -->` between content blocks
- **THEN** content before the first pause is shown initially, and subsequent blocks are revealed one by one on each advance

#### Scenario: Multiple pauses in one slide
- **WHEN** a slide contains N `<!-- pause -->` markers
- **THEN** the slide has N+1 steps, each advance reveals the next content block

#### Scenario: Advance past all pauses proceeds to next slide
- **WHEN** all pause steps in the current slide have been revealed and the user presses Right arrow
- **THEN** the presentation advances to the next slide with a transition effect

#### Scenario: Navigate back resets pause state
- **WHEN** the user navigates back to a slide with pauses
- **THEN** all content is shown (pause state reset to fully revealed)

---

### Requirement: Column Layout
The system SHALL support multi-column layouts within slides via `<!-- column_layout -->`, `<!-- column -->`, and `<!-- reset_layout -->` comment commands.

#### Scenario: Define two-column layout
- **WHEN** a slide contains `<!-- column_layout: [1, 2] -->`
- **THEN** the rendering area is split into two columns with width ratio 1:2

#### Scenario: Write content to specific column
- **WHEN** `<!-- column: 0 -->` is followed by markdown content
- **THEN** the content is rendered within column 0's allocated area

#### Scenario: Reset layout to full width
- **WHEN** `<!-- reset_layout -->` is encountered
- **THEN** subsequent content returns to full-width rendering

#### Scenario: Column layout with image
- **WHEN** one column contains text and another contains `![img](file.pix)`
- **THEN** text and image are rendered side-by-side in their respective column areas

---

### Requirement: Vertical Centering
The system SHALL support `<!-- jump_to_middle -->` to vertically center subsequent content on the slide.

#### Scenario: Center title slide content
- **WHEN** a slide contains `<!-- jump_to_middle -->` before a heading
- **THEN** the heading and subsequent content are vertically centered in the slide area
