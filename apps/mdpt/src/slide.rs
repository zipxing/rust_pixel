use serde::Deserialize;

/// Front matter configuration parsed from YAML header
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FrontMatter {
    pub title: String,
    pub theme: String,
    pub transition: String,
    pub title_animation: String,
    pub code_theme: String,
    pub margin: u16,
}

impl Default for FrontMatter {
    fn default() -> Self {
        Self {
            title: String::new(),
            theme: "dark".to_string(),
            transition: "dissolve".to_string(),
            title_animation: "typewriter".to_string(),
            code_theme: "base16-ocean.dark".to_string(),
            margin: 2,
        }
    }
}

/// A single slide in the presentation
#[derive(Debug, Clone)]
pub struct SlideContent {
    pub elements: Vec<SlideElement>,
    pub transition: Option<String>,
    pub animation: Option<String>,
}

impl SlideContent {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            transition: None,
            animation: None,
        }
    }

    /// Count the number of steps (pause boundaries) in this slide.
    /// A slide with N pause elements has N+1 steps.
    pub fn step_count(&self) -> usize {
        let pause_count = self
            .elements
            .iter()
            .filter(|e| matches!(e, SlideElement::Pause))
            .count();
        pause_count + 1
    }

    /// Get the element index boundary for a given step.
    /// Returns the index up to which elements should be rendered.
    pub fn step_boundary(&self, step: usize) -> usize {
        let mut pause_seen = 0;
        for (i, elem) in self.elements.iter().enumerate() {
            if matches!(elem, SlideElement::Pause) {
                if pause_seen == step {
                    return i;
                }
                pause_seen += 1;
            }
        }
        self.elements.len()
    }
}

/// Individual element within a slide
#[derive(Debug, Clone)]
pub enum SlideElement {
    /// Heading with level (1-6) and text
    Title { level: u8, text: String },

    /// Text paragraph
    Paragraph { text: String },

    /// Fenced code block with optional language and options
    CodeBlock {
        language: String,
        code: String,
        line_numbers: bool,
    },

    /// List (flat representation with depth, like presenterm)
    List { items: Vec<ListItem> },

    /// Table with header and rows
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        alignments: Vec<ColumnAlignment>,
    },

    /// Horizontal rule (thematic break) within a slide
    Divider,

    /// Image reference (.pix or .ssf)
    Image {
        path: String,
        alt: String,
    },

    /// Pause marker — split slide into incremental steps
    Pause,

    /// Define column layout with width ratios
    ColumnLayout { widths: Vec<u32> },

    /// Switch to a specific column
    Column(usize),

    /// Reset to full-width layout
    ResetLayout,

    /// Vertically center subsequent content
    JumpToMiddle,
}

/// A list item with depth for nested lists (flat representation like presenterm)
#[derive(Debug, Clone)]
pub struct ListItem {
    pub text: String,
    pub depth: u8,
    pub ordered: bool,
    pub index: usize,
}

/// Table column alignment
#[derive(Debug, Clone, Copy)]
pub enum ColumnAlignment {
    Left,
    Center,
    Right,
    None,
}

/// The complete parsed presentation
#[derive(Debug, Clone)]
pub struct Presentation {
    pub front_matter: FrontMatter,
    pub slides: Vec<SlideContent>,
}

impl Presentation {
    pub fn new() -> Self {
        Self {
            front_matter: FrontMatter::default(),
            slides: Vec::new(),
        }
    }

    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }
}
