/// MDPT AI Slide Generator
///
/// Calls Gemini API to generate MDPT-compatible markdown presentations,
/// optionally with AI-generated images for each slide.
///
/// Supports both OpenAI-compatible format (base ends with /v1 or /openai)
/// and Gemini native format (generateContent endpoint).
///
/// Configuration is read from `.aikey` file (searched upward from cwd),
/// and can be overridden by environment variables.
///
/// Usage:
///   cargo pixel g "Your topic here"
///   cargo pixel g "Your topic here" --img
///
/// .aikey file format (KEY=VALUE per line):
///   GEMINI_API_KEY=xxx
///   GEMINI_API_BASE=https://aihubmix.com/gemini
///   GEMINI_MODEL=gemini-2.0-flash
///   GEMINI_IMG_MODEL=gemini-3-pro-image-preview

use base64::Engine;
use serde_json::json;
use std::io::Read;

const DEFAULT_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/openai";
const DEFAULT_MODEL: &str = "gemini-2.5-flash-preview";
const DEFAULT_IMG_MODEL: &str = "gemini-3-pro-image-preview";

const SYSTEM_PROMPT: &str = r#"You are an expert presentation designer. You generate Markdown presentations for MDPT (Markdown Presentation Tool).

## MDPT Format Specification

### Frontmatter (YAML, required at top)
```
---
title: Presentation Title
author: Author Name
theme: dark
transition: cycle
code_theme: base16-ocean.dark
margin: 4
transition_duration: 0.2
---
```
- theme: `dark` or `light`
- transition: `cycle`, `fade`, `slide_left`, `slide_right`, `slide_up`, `slide_down`
- code_theme: any syntect theme name like `base16-ocean.dark`, `Solarized (dark)`, `InspiredGitHub`

### Slide Separator
Use `---` on its own line to separate slides.

### Supported Elements

**Headings:**
- `# H1` - main title (use on title/ending slides)
- `## H2` - slide title (most slides use this)
- `### H3` - section heading within a slide

**Lists:**
- Unordered: `* item` or `- item` (supports nesting with indentation)
- Ordered: `1. item`

**Code Blocks with options:**
````
```rust +line_numbers
fn main() {
    println!("Hello!");
}
```
````
Options after language: `+line_numbers`, `+no_background`
Dynamic highlighting: `{1-3|5-8|all}` after language

**Tables:**
```
| Header 1 | Header 2 |
|:---------|:--------:|
| Left     | Center   |
```

**Block Quotes and Alerts:**
```
> Regular quote text

> [!note]
> This is a note alert.

> [!caution]
> This is a caution alert.
```

### Special Comments (HTML comments for MDPT directives)

**Incremental display:**
`<!-- pause -->` - content after this appears on next key press

**Horizontal divider:**
`<!-- divider -->`

**Vertical spacer:**
`<!-- spacer: 2 -->` - adds N blank lines

**Column layout:**
```
<!-- column_layout: [1, 1] -->
<!-- column: 0 -->
Left column content
<!-- column: 1 -->
Right column content
<!-- reset_layout -->
```

**Text animations (one per line of text below it):**
```
<!-- anim: spotlight -->
Text with spotlight animation

<!-- anim: typewriter -->
Text with typewriter animation
```
Available: `spotlight`, `wave`, `fadein`, `typewriter`

**Vertical centering:**
`<!-- jump_to_middle -->` - centers the next content vertically

### Charts (as fenced code blocks)

**Line Chart:**
````
```linechart
title: Monthly Revenue
x: [Jan, Feb, Mar, Apr, May]
y: [120, 200, 150, 300, 280]
y2: [80, 150, 120, 200, 250]
height: 12
```
````

**Bar Chart:**
````
```barchart
title: Programming Languages
labels: [Rust, Go, Python, JS]
values: [95, 72, 88, 78]
height: 14
```
````

**Pie Chart:**
````
```piechart
title: Market Share
labels: [Chrome, Safari, Firefox, Edge]
values: [65, 18, 7, 5]
radius: 20
```
````

**Mermaid Flowchart:**
````
```mermaid
graph TD
A[Start] --> B[Process]
B --> C{Decision}
C -->|Yes| D[Result]
C -->|No| E[Other]
```
````

## Guidelines
1. Generate 8-12 slides with rich content
2. First slide: title slide using `# Title` with a subtitle line
3. Last slide: ending slide using `<!-- jump_to_middle -->` and `# Thank You!`
4. Use varied elements: lists, code, tables, charts, columns, quotes
5. Use `<!-- pause -->` for progressive reveal on content-heavy slides
6. Include at least one code block, one table, and one chart
7. Content should be informative, well-structured, and presentation-ready
8. Use Chinese if the topic is in Chinese, English if in English
9. Output ONLY the raw markdown, no wrapping ```markdown``` fences
"#;

const IMAGE_SYSTEM_PROMPT: &str = r#"You are a professional presentation image designer. Generate a beautiful, high-quality illustration image for a presentation slide.

Requirements:
- Professional, clean, modern design style
- DO NOT include any text, words, letters, numbers, or watermarks in the image
- The image should be a visual illustration or decorative background related to the slide topic
- Use rich colors, good contrast, and visually appealing composition
- The style should be suitable for a professional presentation
- Flat design, vector-style, or modern illustration style preferred
"#;

fn build_user_prompt(topic: &str) -> String {
    format!(
        "Please generate an MDPT markdown presentation about: {}\n\n\
         Requirements:\n\
         - 8-12 slides with substantial content\n\
         - Use a variety of MDPT features (code, tables, charts, columns, animations, etc.)\n\
         - Make it informative and visually engaging\n\
         - Output raw markdown only, no code fences around the entire output",
        topic
    )
}

fn build_image_prompt(slide_content: &str) -> String {
    format!(
        "Generate an illustration image for this presentation slide.\n\n\
         Slide content:\n{}\n\n\
         Create a visually appealing image that represents the key theme of this slide. \
         No text in the image.",
        slide_content
    )
}

fn strip_markdown_fences(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```markdown") {
        if let Some(inner) = rest.strip_suffix("```") {
            return inner.trim().to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("```md") {
        if let Some(inner) = rest.strip_suffix("```") {
            return inner.trim().to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        if let Some(inner) = rest.strip_suffix("```") {
            return inner.trim().to_string();
        }
    }
    trimmed.to_string()
}

/// Parse markdown into slides, skipping YAML frontmatter
fn parse_slides(markdown: &str) -> Vec<String> {
    let content = if markdown.starts_with("---") {
        if let Some(end) = markdown[3..].find("\n---") {
            &markdown[3 + end + 4..]
        } else {
            markdown
        }
    } else {
        markdown
    };

    content
        .split("\n---")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Extract a short title/summary from slide content for logging
fn slide_title(slide: &str) -> String {
    for line in slide.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            return line.trim_start_matches('#').trim().to_string();
        }
    }
    for line in slide.lines() {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with("<!--") {
            let truncated: String = line.chars().take(40).collect();
            return truncated;
        }
    }
    "(empty slide)".to_string()
}

/// Extract base64 image data from response content.
/// Looks for patterns like: data:image/png;base64,xxxxx
fn extract_base64_image(content: &str) -> Option<(String, String)> {
    // Pattern: data:image/(png|jpeg|jpg|gif|webp);base64,<data>
    let prefix = "data:image/";
    let mut search_from = 0;
    while let Some(start) = content[search_from..].find(prefix) {
        let abs_start = search_from + start;
        let after_prefix = &content[abs_start + prefix.len()..];

        // Find mime subtype and ;base64,
        if let Some(semi_pos) = after_prefix.find(";base64,") {
            let mime_sub = &after_prefix[..semi_pos];
            let b64_start = abs_start + prefix.len() + semi_pos + ";base64,".len();

            // Collect base64 chars
            let b64_data: String = content[b64_start..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/' || *c == '=')
                .collect();

            if b64_data.len() > 100 {
                let ext = match mime_sub {
                    "jpeg" | "jpg" => "jpg",
                    "gif" => "gif",
                    "webp" => "webp",
                    _ => "png",
                };
                return Some((b64_data, ext.to_string()));
            }
        }
        search_from = abs_start + 1;
    }
    None
}

/// Extract image URL from markdown image syntax: ![...](url)
fn extract_image_url(content: &str) -> Option<String> {
    let marker = "](";
    let mut search_from = 0;
    while let Some(pos) = content[search_from..].find(marker) {
        let abs_pos = search_from + pos;
        // Check there's a ! and [ before
        if abs_pos > 0 {
            let before = &content[..abs_pos];
            if let Some(bracket_pos) = before.rfind("![") {
                let url_start = abs_pos + marker.len();
                if let Some(paren_end) = content[url_start..].find(')') {
                    let url = content[url_start..url_start + paren_end].trim();
                    if url.starts_with("http") && !url.contains(' ') {
                        return Some(url.to_string());
                    }
                }
                let _ = bracket_pos;
            }
        }
        search_from = abs_pos + 1;
    }
    None
}

/// Download image from URL, return bytes
fn download_image(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("Failed to download image: {}", e))?;

    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;

    Ok(bytes)
}

/// Detect if the API base URL uses OpenAI-compatible format.
/// URLs ending with /v1 or containing /openai use OpenAI format;
/// otherwise use Gemini native format.
fn is_openai_format(api_base: &str) -> bool {
    let base = api_base.trim_end_matches('/');
    base.ends_with("/v1") || base.ends_with("/openai")
}

/// Build Gemini native API URL: {base}/v1beta/models/{model}:generateContent
/// If base already contains /v1beta, just append /models/{model}:generateContent
fn gemini_url(api_base: &str, model: &str) -> String {
    let base = api_base.trim_end_matches('/');
    if base.contains("/v1beta") {
        format!("{}/models/{}:generateContent", base, model)
    } else {
        format!("{}/v1beta/models/{}:generateContent", base, model)
    }
}

/// Send HTTP POST, return parsed JSON or error with body.
/// OpenAI format uses Bearer token; Gemini native uses ?key= query param.
fn api_post(url: &str, api_key: &str, body: &serde_json::Value, openai: bool) -> Result<serde_json::Value, String> {
    let real_url = if openai {
        url.to_string()
    } else {
        format!("{}?key={}", url, api_key)
    };
    eprintln!("[DEBUG] POST {}", url);

    let mut req = ureq::post(&real_url);
    if openai {
        req = req.set("Authorization", &format!("Bearer {}", api_key));
    }

    let response = match req.send_json(body) {
        Ok(resp) => resp,
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            eprintln!("[DEBUG] HTTP {} response body:\n{}", code, body);
            return Err(format!("HTTP {}: {}", code, body));
        }
        Err(e) => {
            return Err(format!("HTTP request failed: {}", e));
        }
    };

    response
        .into_json()
        .map_err(|e| format!("Failed to parse response JSON: {}", e))
}

/// Extract text content from API response (handles both OpenAI and Gemini formats)
fn extract_text(resp: &serde_json::Value, openai: bool) -> Result<String, String> {
    let text = if openai {
        resp["choices"][0]["message"]["content"].as_str()
    } else {
        // Gemini native: candidates[0].content.parts[0].text
        resp["candidates"][0]["content"]["parts"][0]["text"].as_str()
    };
    text.map(|s| s.to_string()).ok_or_else(|| {
        format!(
            "Unexpected response format: {}",
            serde_json::to_string_pretty(resp).unwrap_or_default()
        )
    })
}

fn call_gemini(api_key: &str, api_base: &str, model: &str, topic: &str) -> Result<String, String> {
    let openai = is_openai_format(api_base);

    let (url, body) = if openai {
        let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
        let body = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": build_user_prompt(topic)}
            ]
        });
        (url, body)
    } else {
        let url = gemini_url(api_base, model);
        let body = json!({
            "contents": [
                {"role": "user", "parts": [{"text": build_user_prompt(topic)}]}
            ],
            "systemInstruction": {
                "parts": [{"text": SYSTEM_PROMPT}]
            }
        });
        (url, body)
    };

    eprintln!("[DEBUG] Model: {} ({})", model, if openai { "openai" } else { "gemini-native" });

    let resp = api_post(&url, api_key, &body, openai)?;
    let content = extract_text(&resp, openai)?;
    Ok(strip_markdown_fences(&content))
}

/// Save base64 image data to file, return filename
fn save_b64_image(b64_data: &str, ext: &str, slide_index: usize, output_dir: &std::path::Path) -> Result<String, String> {
    let filename = format!("slide_{}.{}", slide_index, ext);
    let filepath = output_dir.join(&filename);
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(b64_data)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;
    std::fs::write(&filepath, &decoded)
        .map_err(|e| format!("Failed to write {}: {}", filepath.display(), e))?;
    Ok(filename)
}

/// Generate an image for a slide. Supports both OpenAI-compatible and Gemini native formats.
fn generate_slide_image(
    api_key: &str,
    api_base: &str,
    img_model: &str,
    slide_content: &str,
    slide_index: usize,
    output_dir: &std::path::Path,
) -> Result<String, String> {
    let openai = is_openai_format(api_base);
    let prompt = build_image_prompt(slide_content);

    let (url, body) = if openai {
        let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
        let body = json!({
            "model": img_model,
            "messages": [
                {"role": "system", "content": IMAGE_SYSTEM_PROMPT},
                {"role": "user", "content": prompt}
            ]
        });
        (url, body)
    } else {
        let url = gemini_url(api_base, img_model);
        let body = json!({
            "contents": [
                {"role": "user", "parts": [{"text": prompt}]}
            ],
            "systemInstruction": {
                "parts": [{"text": IMAGE_SYSTEM_PROMPT}]
            },
            "generationConfig": {
                "responseModalities": ["TEXT", "IMAGE"]
            }
        });
        (url, body)
    };

    let resp = api_post(&url, api_key, &body, openai)?;

    // For Gemini native: check inlineData in parts first
    if !openai {
        if let Some(parts) = resp["candidates"][0]["content"]["parts"].as_array() {
            for part in parts {
                if let Some(inline) = part.get("inlineData") {
                    if let (Some(mime), Some(data)) = (
                        inline["mimeType"].as_str(),
                        inline["data"].as_str(),
                    ) {
                        let ext = match mime {
                            "image/jpeg" | "image/jpg" => "jpg",
                            "image/gif" => "gif",
                            "image/webp" => "webp",
                            _ => "png",
                        };
                        return save_b64_image(data, ext, slide_index, output_dir);
                    }
                }
            }
        }
    }

    // Fallback: extract from text content (works for both formats)
    let content = extract_text(&resp, openai).unwrap_or_default();

    // Try base64 data URL in text
    if let Some((b64_data, ext)) = extract_base64_image(&content) {
        return save_b64_image(&b64_data, &ext, slide_index, output_dir);
    }

    // Try markdown image URL
    if let Some(img_url) = extract_image_url(&content) {
        let bytes = download_image(&img_url)?;
        let ext = if img_url.contains(".jpg") || img_url.contains(".jpeg") { "jpg" } else { "png" };
        let filename = format!("slide_{}.{}", slide_index, ext);
        let filepath = output_dir.join(&filename);
        std::fs::write(&filepath, &bytes)
            .map_err(|e| format!("Failed to write {}: {}", filepath.display(), e))?;
        return Ok(filename);
    }

    Err(format!(
        "No image data in response (content length: {} chars)",
        content.len()
    ))
}

/// Load KEY=VALUE pairs from `.aikey` file, searching upward from cwd.
/// Values are set as environment variables only if not already set
/// (env vars take precedence over file).
fn load_aikey_file() {
    let mut dir = std::env::current_dir().ok();
    while let Some(d) = dir {
        let keyfile = d.join(".aikey");
        if keyfile.is_file() {
            if let Ok(content) = std::fs::read_to_string(&keyfile) {
                eprintln!("Loaded config from {}", keyfile.display());
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        let value = value.trim();
                        // Only set if not already in env (env takes precedence)
                        if std::env::var(key).is_err() {
                            std::env::set_var(key, value);
                        }
                    }
                }
            }
            return;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
}

fn main() {
    // Load .aikey config (env vars take precedence)
    load_aikey_file();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: gen <topic> [--img]");
        eprintln!("  --img  Also generate images for each slide");
        eprintln!();
        eprintln!("  Example: cargo pixel g \"Rust语言入门\"");
        eprintln!("  Example: cargo pixel g \"Rust语言入门\" --img");
        std::process::exit(1);
    }

    let topic = &args[1];
    let gen_images = args.iter().any(|a| a == "--img");

    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| {
        eprintln!("Error: GEMINI_API_KEY not found in .aikey file or environment");
        std::process::exit(1);
    });

    let api_base =
        std::env::var("GEMINI_API_BASE").unwrap_or_else(|_| DEFAULT_API_BASE.to_string());
    let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    eprintln!("Generating presentation for: \"{}\"", topic);

    match call_gemini(&api_key, &api_base, &model, topic) {
        Ok(markdown) => {
            let out_path = {
                let manifest_dir = env!("CARGO_MANIFEST_DIR");
                let p = std::path::PathBuf::from(manifest_dir)
                    .join("assets")
                    .join("gen.md");
                if p.parent().map(|d| d.exists()).unwrap_or(false) {
                    p
                } else {
                    std::path::PathBuf::from("gen.md")
                }
            };

            std::fs::write(&out_path, &markdown).unwrap_or_else(|e| {
                eprintln!("Failed to write {}: {}", out_path.display(), e);
                std::process::exit(1);
            });

            let slide_count = markdown.matches("\n---").count() + 1;
            eprintln!(
                "Generated {} slides -> {}",
                slide_count,
                out_path.display()
            );

            // Image generation (uses same API base, OpenAI-compatible format)
            if gen_images {
                let img_model = std::env::var("GEMINI_IMG_MODEL")
                    .unwrap_or_else(|_| DEFAULT_IMG_MODEL.to_string());

                let output_dir = std::path::PathBuf::from("tmp/mdpt/aiimg");
                std::fs::create_dir_all(&output_dir).unwrap_or_else(|e| {
                    eprintln!("Failed to create {}: {}", output_dir.display(), e);
                    std::process::exit(1);
                });

                let slides = parse_slides(&markdown);
                eprintln!(
                    "\nGenerating images for {} slides (model={})...",
                    slides.len(),
                    img_model
                );

                let mut success = 0;
                for (i, slide) in slides.iter().enumerate() {
                    let idx = i + 1;
                    let title = slide_title(slide);
                    eprint!("  [{}/{}] \"{}\" ... ", idx, slides.len(), title);

                    match generate_slide_image(
                        &api_key,
                        &api_base,
                        &img_model,
                        slide,
                        idx,
                        &output_dir,
                    ) {
                        Ok(filename) => {
                            eprintln!("-> {}", filename);
                            success += 1;
                        }
                        Err(e) => {
                            eprintln!("FAILED: {}", e);
                        }
                    }
                }

                eprintln!(
                    "\nImage generation complete: {}/{} succeeded -> {}",
                    success,
                    slides.len(),
                    output_dir.display()
                );
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
