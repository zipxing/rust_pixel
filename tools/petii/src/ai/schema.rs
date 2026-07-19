use serde::{Deserialize, Serialize};

const MAX_TEXT_BYTES: usize = 4096;

/// Art direction returned alongside a generated reference image. It is the only structured value the
/// provider produces; the PETSCII conversion that follows is fully deterministic and consumes no
/// model output, so there is no critique/repair schema to validate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtPlan {
    pub summary: String,
    pub reference_prompt: String,
    #[serde(default)]
    pub palette: Vec<u8>,
}

impl ArtPlan {
    pub fn validate(&self) -> Result<(), String> {
        validate_text("summary", &self.summary)?;
        validate_text("reference_prompt", &self.reference_prompt)?;
        if self.palette.len() > 16 {
            return Err("art plan palette exceeds 16 colors".to_string());
        }
        Ok(())
    }
}

fn validate_text(name: &str, text: &str) -> Result<(), String> {
    if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
        return Err(format!("{name} must contain 1..={MAX_TEXT_BYTES} bytes"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_plan_accepts_valid_fields() {
        let plan = ArtPlan {
            summary: "a moonlit witch".to_string(),
            reference_prompt: "a moonlit witch on a plain backdrop".to_string(),
            palette: (0..16).collect(),
        };
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn art_plan_rejects_empty_text_and_oversized_palette() {
        let empty = ArtPlan {
            summary: "  ".to_string(),
            reference_prompt: "x".to_string(),
            palette: vec![],
        };
        assert!(empty.validate().is_err());

        let big_palette = ArtPlan {
            summary: "ok".to_string(),
            reference_prompt: "ok".to_string(),
            palette: (0..20).collect(),
        };
        assert!(big_palette.validate().is_err());
    }
}
