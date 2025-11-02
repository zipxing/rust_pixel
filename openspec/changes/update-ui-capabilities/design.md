## Context
This project provides a Rust-based UI framework (`src/ui`, `src/render`, `src/event`) used across multiple apps. Current widgets (e.g., `button`, `label`, `list`, `textbox`) cover basics but lack advanced components, strong focus/keyboard handling, runtime theming, event phasing, and large-list performance. The goal is additive improvements without breaking existing APIs.

## Goals / Non-Goals
- Goals:
  - Add core components: Tabs, Modal/Dialog, Tooltip, Dropdown/Select, Slider, ProgressBar, ToggleSwitch, Checkbox, Radio, Toast/Notification, Image/Icon
  - Add layout primitives: Stack (H/V), Grid
  - Runtime theming & component variants; high-contrast support
  - Unified focus management & keyboard navigation; visible focus indicator
  - Event bubbling/capture with stop propagation & prevent default semantics
  - Virtualized long list; batched updates and dirty-region rendering
  - Accessibility roles/labels and consistent disabled/readonly semantics
- Non-Goals:
  - No breaking changes to existing widget APIs
  - No test-only components or demo-only code in the main tree
  - No professional desktop GUI parity (windowing/complex typography/anti-aliased vector widget drawing)
  - No per-pixel bespoke visuals beyond existing sprite/atlas usage; keep character-cell first
- Rendering Model & Cell Canonicalization
  - Character-cell is the canonical coordinate system for both terminal and graphics backends
  - Graphics backends reuse the same symbol/texture atlas to approximate terminal visuals
  - Avoid per-pixel text shaping and subpixel metrics; monospace-first assumptions in UI

## Decisions
- Event Phases
  - Introduce capture → target → bubble phases for input events
  - Add flags: `stop_propagation`, `prevent_default` observed by subsequent handlers
  - Keep event structs lightweight and re-usable; store phase/flags in event context

- Focus Management
  - Maintain a focus tree aligned with widget hierarchy
  - Tab/Shift+Tab traversal order comes from layout order, skipping disabled/readonly
  - Arrow-key navigation for lists/menus/tabs configured per widget capability
  - Central `FocusManager` publishes focus-changed events for visuals and a11y

- Theming & Variants
  - Theme object stored in UI `Context`; switching theme triggers style recompute
  - Component variants (primary/secondary/ghost) map to style tokens (colors, borders, sizes)
  - High-contrast theme inherits tokens with contrast-aware overrides

- Layout Primitives
  - Stack (H/V): alignment (start/center/end), spacing, stretch rules
  - Grid: fixed/flex columns and rows, gaps, child placement with span support

- Virtualized List
  - Render visible window + overscan (configurable)
  - Maintain item height strategy: fixed-height first; pluggable estimator later
  - Recycle item widgets where possible; stable keys for state preservation

- Rendering Optimization
  - Batch state updates during a frame; schedule a single render pass
  - Dirty-region tracking in render layer; re-draw minimal areas
  - Avoid unnecessary re-layout: diff layout inputs before recomputing

- Widget Lifecycle & State
  - Widgets expose controlled/unchecked states consistently (e.g., Checkbox/Radio)
  - Modal traps focus while open; closing restores prior focus
  - Tooltip/Dropdown/Toast/Modal render on a top layer; auto-hide timers configured

- Keyboard & Shortcuts
  - Normalize key events; provide per-widget handlers and global accelerator map
  - Respect prevent_default for text fields and navigation conflicts

- Accessibility
  - Assign roles (button, tab, tablist, dialog, tooltip, listbox, slider, progressbar)
  - Provide labels/aria-like metadata in a minimal cross-platform struct
  - Ensure keyboard equivalence for all interactive features

- API Shape & Compatibility
  - New types live under `src/ui/components` and layout under `src/ui/layout`
  - Existing components gain optional variant/focus props; defaults preserve behavior
  - No renames/removals in public modules in this change

## Risks / Trade-offs
- Complexity: Event phases and focus tree add moving parts → mitigate with clear docs and invariants
- Performance: Virtualization heuristics may cause pop-in → mitigate with overscan and smooth estimates
- Theming churn: Runtime switch can cause wide invalidation → mitigate with batched recompute and diffing

## Migration Plan
1. Land infrastructure (event phases, focus manager, theme tokens)
2. Add layout primitives (Stack/Grid)
3. Introduce new components in small batches (Tabs/Modal/Tooltip → Dropdown/Slider/Progress → Toggle/Checkbox/Radio → Toast/Image/Icon)
4. Integrate virtualization into List as opt-in feature
5. Update existing components to accept variants and focus visuals (backward compatible)

## Open Questions
- Do we need per-platform theme token overrides beyond contrast? (e.g., platform fonts)
- Should List support variable item heights in first iteration?
- Which reference machine defines the frame-time target for performance scenarios?


