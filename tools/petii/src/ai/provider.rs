use crate::ai::schema::{ArtPlan, Critique};
use base64::Engine;
use image::DynamicImage;
use serde_json::{json, Value};
use std::{io::Read, time::Duration};

const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;

/// Provider-neutral boundary for prompt-to-reference generation.
pub trait ReferenceGenerator {
    fn generate_reference(
        &self,
        prompt: &str,
        width: u32,
        height: u32,
    ) -> Result<(ArtPlan, DynamicImage), String>;
}

/// Provider-neutral boundary for multimodal candidate critique.
pub trait MultimodalCritic {
    fn critique(
        &self,
        prompt: &str,
        reference: &DynamicImage,
        candidates: &[DynamicImage],
        grid_width: u32,
        grid_height: u32,
        allowed_colors: &[u8],
    ) -> Result<Critique, String>;
}

/// Minimal OpenAI-compatible adapter. It intentionally targets only the image
/// generation and chat-completions shapes needed by this experimental tool.
pub struct OpenAiCompatibleProvider {
    api_base: String,
    api_key: String,
    image_model: String,
    vision_model: String,
    agent: ureq::Agent,
    max_retries: u8,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        api_base: impl Into<String>,
        api_key: impl Into<String>,
        image_model: impl Into<String>,
        vision_model: impl Into<String>,
    ) -> Result<Self, String> {
        let api_base = api_base.into().trim_end_matches('/').to_string();
        let api_key = api_key.into();
        if api_base.is_empty() || api_key.is_empty() {
            return Err("API base and key must be non-empty".to_string());
        }
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(90))
            .timeout_write(Duration::from_secs(30))
            .build();
        Ok(Self {
            api_base,
            api_key,
            image_model: image_model.into(),
            vision_model: vision_model.into(),
            agent,
            max_retries: 1,
        })
    }

    pub fn from_env() -> Result<Self, String> {
        let api_base = std::env::var("PETII_AI_API_BASE")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let api_key = std::env::var("PETII_AI_API_KEY")
            .map_err(|_| "PETII_AI_API_KEY is not configured".to_string())?;
        let image_model =
            std::env::var("PETII_AI_IMAGE_MODEL").unwrap_or_else(|_| "gpt-image-2".to_string());
        let vision_model =
            std::env::var("PETII_AI_VISION_MODEL").unwrap_or_else(|_| "gpt-4.1-mini".to_string());
        Self::new(api_base, api_key, image_model, vision_model)
    }

    fn post_json(&self, endpoint: &str, body: &Value) -> Result<Value, String> {
        let url = format!("{}/{}", self.api_base, endpoint.trim_start_matches('/'));
        let mut last_error = String::new();
        for attempt in 0..=self.max_retries {
            let response = self
                .agent
                .post(&url)
                .set("Authorization", &format!("Bearer {}", self.api_key))
                .set("Content-Type", "application/json")
                .send_json(body.clone());
            match response {
                Ok(response) => return read_bounded_json(response),
                Err(ureq::Error::Status(status, response)) => {
                    let body = read_bounded_text(response).unwrap_or_default();
                    last_error = format!("provider HTTP {status}: {}", truncate(&body, 1024));
                    if status < 500 && status != 429 {
                        break;
                    }
                }
                Err(error) => last_error = format!("provider request failed: {error}"),
            }
            if attempt < self.max_retries {
                std::thread::sleep(Duration::from_millis(250 * (attempt as u64 + 1)));
            }
        }
        Err(last_error)
    }
}

impl ReferenceGenerator for OpenAiCompatibleProvider {
    fn generate_reference(
        &self,
        prompt: &str,
        width: u32,
        height: u32,
    ) -> Result<(ArtPlan, DynamicImage), String> {
        if prompt.trim().is_empty() || prompt.len() > 4096 {
            return Err("prompt must contain 1..=4096 bytes".to_string());
        }
        let reference_prompt = reference_art_direction(prompt, width, height);
        let body = image_generation_request(&self.image_model, &reference_prompt);
        let response = self.post_json("images/generations", &body)?;
        let encoded = response["data"][0]["b64_json"]
            .as_str()
            .ok_or_else(|| "image response does not contain data[0].b64_json".to_string())?;
        if encoded.len() > (MAX_RESPONSE_BYTES as usize * 2) {
            return Err("base64 image response exceeds the configured bound".to_string());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| format!("invalid base64 image response: {error}"))?;
        let image = image::load_from_memory(&bytes)
            .map_err(|error| format!("invalid generated image: {error}"))?;
        let plan = ArtPlan {
            summary: prompt.trim().to_string(),
            reference_prompt,
            palette: (0..16).collect(),
            protected_regions: Vec::new(),
        };
        plan.validate()?;
        Ok((plan, image))
    }
}

/// Structured art direction for a conversion-friendly reference. The template asks for the exact
/// properties the deterministic converter renders well — crisp continuous contours, large coherent
/// color regions, and a few intentional diagonal/curved edges — while forbidding the high-frequency
/// content (gradients, noise, dithering, tiny details) it cannot represent. The subject is the only
/// per-request field; every other line is a fixed constraint.
fn reference_art_direction(prompt: &str, width: u32, height: u32) -> String {
    let subject = prompt.trim();
    format!(
        "Use case: stylized-concept\n\
         Asset type: reference image for PETSCII conversion\n\
         Primary request: {subject}\n\
         Scene/backdrop: one clear subject on a plain, uncluttered backdrop; no extra objects.\n\
         Style/medium: clean flat-color retro editorial illustration, not pixel art and not PETSCII.\n\
         Composition/framing: {width}:{height} canvas; large centered subject; generous outer margin; readable at {width}x{height} cells.\n\
         Lighting/mood: strong directional rim light with clear separation between subject and background.\n\
         Color palette: 6 to 8 solid colors, high contrast but harmonious.\n\
         Constraints: crisp continuous contours; large coherent color regions; a few intentional diagonal and curved edges; no tiny details.\n\
         Avoid: text, watermark, frame, gradients, noise, dithering, halftone, photorealism, pixel art, ASCII art, PETSCII."
    )
}

fn image_generation_request(model: &str, prompt: &str) -> Value {
    // GPT Image responses contain `data[0].b64_json` by default. In particular,
    // GPT Image 2 rejects the older explicit `response_format` parameter.
    json!({
        "model": model,
        "prompt": prompt,
        "size": "1024x1024"
    })
}

impl MultimodalCritic for OpenAiCompatibleProvider {
    fn critique(
        &self,
        prompt: &str,
        reference: &DynamicImage,
        candidates: &[DynamicImage],
        grid_width: u32,
        grid_height: u32,
        allowed_colors: &[u8],
    ) -> Result<Critique, String> {
        if candidates.is_empty() || candidates.len() > 8 {
            return Err("critic requires 1..=8 candidates".to_string());
        }
        let mut content = vec![json!({
            "type": "text",
            "text": critique_prompt(prompt, grid_width, grid_height, allowed_colors)
        })];
        content.push(image_content("Reference image", reference)?);
        for (index, candidate) in candidates.iter().enumerate() {
            content.push(image_content(&format!("Candidate {index}"), candidate)?);
        }
        let body = critic_request(&self.vision_model, content);
        let response = self.post_json("chat/completions", &body)?;
        let text = response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| "critic response does not contain message content".to_string())?;
        let critique = Critique::from_json(text, grid_width, grid_height, allowed_colors)?;
        critique.validate_candidate_count(candidates.len())?;
        Ok(critique)
    }
}

fn critic_request(model: &str, content: Vec<Value>) -> Value {
    let mut body = json!({
        "model": model,
        "response_format": {"type": "json_object"},
        "messages": [{"role": "user", "content": content}]
    });
    // GPT-5-family Chat Completions currently accepts only the default
    // temperature. Older multimodal models still benefit from deterministic 0.
    if !model.starts_with("gpt-5") {
        body["temperature"] = json!(0);
    }
    body
}

fn image_content(label: &str, image: &DynamicImage) -> Result<Value, String> {
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .map_err(|error| format!("failed to encode {label}: {error}"))?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(format!("{label} exceeds the image-size bound"));
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(json!({
        "type": "image_url",
        "image_url": {"url": format!("data:image/png;base64,{encoded}")}
    }))
}

fn critique_prompt(prompt: &str, width: u32, height: u32, colors: &[u8]) -> String {
    format!(
        "Evaluate the PETSCII candidate against the reference and intent: {prompt}\n\
         Grid: {width}x{height}; allowed palette indices: {colors:?}.\n\
         Return JSON only with keys: selected_candidate, scores, regions, repairs, explanation.\n\
         selected_candidate must be the zero-based index of the strongest candidate.\n\
         scores must contain semantic_fidelity, subject_readability, composition, \
         palette_coherence, contour_continuity, petscii_authenticity (0..100).\n\
         Regions use normalized x,y,width,height. Prefer bounded repairs: simplify_region, \
         protect_silhouette, reduce_density, increase_contrast. Do not request custom images \
         or characters outside the fixed PETSCII set."
    )
}

fn read_bounded_json(response: ureq::Response) -> Result<Value, String> {
    let text = read_bounded_text(response)?;
    serde_json::from_str(&text).map_err(|error| format!("invalid provider JSON: {error}"))
}

fn read_bounded_text(response: ureq::Response) -> Result<String, String> {
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read provider response: {error}"))?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        return Err("provider response exceeds 16 MiB".to_string());
    }
    String::from_utf8(bytes).map_err(|error| format!("provider response is not UTF-8: {error}"))
}

fn truncate(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn image_content_is_bounded_data_url() {
        let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(1, 1, Rgba([1, 2, 3, 255])));
        let value = image_content("test", &image).unwrap();
        assert!(value["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));
    }

    #[test]
    fn reference_art_direction_embeds_subject_and_conversion_constraints() {
        let prompt = reference_art_direction("  a moonlit witch  ", 40, 25);
        assert!(prompt.contains("Primary request: a moonlit witch"));
        assert!(prompt.contains("readable at 40x25 cells"));
        assert!(prompt.contains("crisp continuous contours"));
        assert!(prompt.contains("intentional diagonal and curved edges"));
        assert!(prompt.contains("Avoid:") && prompt.contains("gradients"));
        assert!(prompt.contains("not pixel art and not PETSCII"));
    }

    #[test]
    fn gpt_image_2_request_omits_legacy_response_format() {
        let body = image_generation_request("gpt-image-2", "a moonlit lion");
        assert_eq!(body["model"], "gpt-image-2");
        assert_eq!(body["size"], "1024x1024");
        assert!(body.get("response_format").is_none());
    }

    #[test]
    fn gpt_5_critic_request_uses_default_temperature() {
        let body = critic_request("gpt-5.6-sol", vec![json!({"type": "text"})]);
        assert!(body.get("temperature").is_none());
        assert_eq!(body["response_format"]["type"], "json_object");
    }

    #[test]
    fn legacy_critic_request_keeps_zero_temperature() {
        let body = critic_request("gpt-4.1-mini", vec![json!({"type": "text"})]);
        assert_eq!(body["temperature"], 0);
    }

    #[test]
    fn empty_credentials_are_rejected() {
        assert!(OpenAiCompatibleProvider::new("", "key", "image", "vision").is_err());
        assert!(
            OpenAiCompatibleProvider::new("https://example.test/v1", "", "image", "vision")
                .is_err()
        );
    }
}
