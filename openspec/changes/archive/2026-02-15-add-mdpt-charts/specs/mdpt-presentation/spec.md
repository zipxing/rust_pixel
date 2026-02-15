## ADDED Requirements

### Requirement: Line Chart Rendering
The system SHALL support rendering line charts from text data descriptions in markdown code blocks tagged with `linechart`, using Unicode Braille characters for high-resolution character-based plotting.

#### Scenario: Render single-series line chart
- **WHEN** a slide contains a `linechart` code block with `x` labels and `y` values
- **THEN** a character-based line chart is rendered with Braille dot-matrix plotting, X-axis labels at the bottom, Y-axis scale on the left, and an optional title at the top

#### Scenario: Render multi-series line chart
- **WHEN** a `linechart` code block contains multiple y-series (`y`, `y2`, `y3`, etc.)
- **THEN** each series is plotted in a distinct color from the chart color palette, and a legend is displayed showing series names

#### Scenario: Auto-scale chart axes
- **WHEN** line chart data values range from min to max
- **THEN** the Y-axis is automatically scaled to fit the data range with appropriate tick marks, and the X-axis distributes labels evenly across the chart width

#### Scenario: Configurable chart dimensions
- **WHEN** a `linechart` code block specifies `width` and/or `height` parameters
- **THEN** the chart renders at the specified character dimensions; if omitted, the chart uses the available slide width and a default height of 15 rows

---

### Requirement: Bar Chart Rendering
The system SHALL support rendering bar charts (histograms) from text data descriptions in markdown code blocks tagged with `barchart`, using Unicode Block Element characters for fractional-height bars.

#### Scenario: Render vertical bar chart
- **WHEN** a slide contains a `barchart` code block with `labels` and `values`
- **THEN** vertical bars are rendered using Block Element characters (▁▂▃▄▅▆▇█) with 1/8 height precision, each bar labeled below and value displayed above

#### Scenario: Color-coded bars
- **WHEN** a bar chart has multiple bars
- **THEN** each bar is rendered in a distinct color from the chart color palette, cycling through colors if there are more bars than palette entries

#### Scenario: Auto-scale bar heights
- **WHEN** bar chart values range from min to max
- **THEN** bars are scaled so the maximum value fills the chart height, and a Y-axis with scale markings is displayed on the left

#### Scenario: Configurable bar chart dimensions
- **WHEN** a `barchart` code block specifies `width` and/or `height` parameters
- **THEN** the chart renders at the specified dimensions; bars are evenly spaced within the available width

---

### Requirement: Pie Chart Rendering
The system SHALL support rendering pie charts from text data descriptions in markdown code blocks tagged with `piechart`, using Unicode Braille characters to draw circular sector graphics.

#### Scenario: Render pie chart with labeled sectors
- **WHEN** a slide contains a `piechart` code block with `labels` and `values`
- **THEN** a circular pie chart is rendered using Braille dot-matrix, with each sector sized proportionally to its value and colored distinctly

#### Scenario: Display pie chart legend
- **WHEN** a pie chart has labeled sectors
- **THEN** a legend is displayed to the right of the chart showing each label with its color indicator and percentage

#### Scenario: Configurable pie chart radius
- **WHEN** a `piechart` code block specifies a `radius` parameter
- **THEN** the chart circle is rendered at the specified radius (in character rows); if omitted, a default radius of 8 is used

#### Scenario: Handle small sector values
- **WHEN** a pie chart sector represents less than 2% of the total
- **THEN** the sector is still rendered with a minimum visible arc, and its label appears in the legend with the actual percentage

---

### Requirement: Mermaid Flowchart Rendering (Minimal Subset)
The system SHALL support parsing a minimal subset of Mermaid `graph` syntax from markdown code blocks tagged with `mermaid`, rendering simple directed flowcharts as character-based box-and-line drawings.

#### Scenario: Render mermaid graph (top-down)
- **WHEN** a slide contains a `mermaid` code block with `graph TD` syntax defining nodes (`A[text]`) and edges (`A --> B`)
- **THEN** nodes are rendered as box-drawing rectangles, connected by vertical line characters with arrow indicators, using simple hierarchical layout

#### Scenario: Render mermaid graph (left-right)
- **WHEN** a `mermaid` code block uses `graph LR` direction
- **THEN** the graph is laid out horizontally with nodes arranged left-to-right, connected by horizontal arrows

#### Scenario: Display edge labels
- **WHEN** a mermaid edge definition includes a label (e.g., `A -->|label| B`)
- **THEN** the label text is displayed along the connecting line at the midpoint

#### Scenario: Fallback for unsupported mermaid syntax
- **WHEN** a `mermaid` code block contains unsupported syntax (e.g., sequenceDiagram, classDiagram, subgraph, style)
- **THEN** the block is rendered as a plain code block with the original mermaid source text displayed
