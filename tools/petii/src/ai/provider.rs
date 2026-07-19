use crate::ai::schema::ArtPlan;
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

/// Minimal OpenAI-compatible adapter. Its only job is prompt-to-reference image
/// generation; the PETSCII conversion that follows is fully deterministic and
/// never calls a model, so no chat/vision endpoint is used.
pub struct OpenAiCompatibleProvider {
    api_base: String,
    api_key: String,
    image_model: String,
    agent: ureq::Agent,
    max_retries: u8,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        api_base: impl Into<String>,
        api_key: impl Into<String>,
        image_model: impl Into<String>,
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
        Self::new(api_base, api_key, image_model)
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
         Scene/backdrop: ONE clear subject on a plain, uncluttered backdrop; no extra objects.\n\
         Style/medium: clean flat-color retro editorial illustration, not pixel art and not PETSCII.\n\
         Composition/framing: {width}:{height} canvas; ONE large centered subject; generous outer margin; the whole image is only {width}x{height} coarse blocks, so every shape must stay legible at that tiny resolution.\n\
         Detail level: MINIMAL. Use only a few large, simple, well-separated shapes. Every meaningful feature must be at least ~3 blocks wide; merge small parts into single bold shapes and drop anything smaller.\n\
         Lighting/mood: strong directional rim light with clear separation between subject and background.\n\
         Color palette: 6 to 8 solid colors, high contrast but harmonious.\n\
         Outlines: give the subject and each major shape a clean, even, dark outline like a storybook or cel illustration — clear and continuous so the shape reads at a glance, but thin and restrained: never thick, heavy, sketchy, doubled, or exaggerated.\n\
         Constraints: crisp continuous contours; large coherent color regions; a few intentional diagonal and curved edges.\n\
         Avoid: fine detail and small repeated elements (individual leaves, petals, flowers, bricks, cobblestones, blades of grass, hair strands, distant or background objects, small ornaments); busy or cluttered scenes; text, watermark, frame, gradients, noise, dithering, halftone, photorealism, pixel art, ASCII art, PETSCII."
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

    #[test]
    fn reference_art_direction_embeds_subject_and_conversion_constraints() {
        let prompt = reference_art_direction("  a moonlit witch  ", 40, 25);
        assert!(prompt.contains("Primary request: a moonlit witch"));
        assert!(prompt.contains("40x25 coarse blocks"));
        assert!(prompt.contains("Detail level: MINIMAL"));
        assert!(prompt.contains("Outlines:") && prompt.contains("never thick, heavy, sketchy"));
        assert!(prompt.contains("crisp continuous contours"));
        assert!(prompt.contains("intentional diagonal and curved edges"));
        assert!(prompt.contains("Avoid:") && prompt.contains("gradients"));
        assert!(prompt.contains("small repeated elements"));
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
    fn empty_credentials_are_rejected() {
        assert!(OpenAiCompatibleProvider::new("", "key", "image").is_err());
        assert!(OpenAiCompatibleProvider::new("https://example.test/v1", "", "image").is_err());
    }
}
