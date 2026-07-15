use serde::{Deserialize, Serialize};

const MAX_TEXT_BYTES: usize = 4096;
const MAX_REGIONS: usize = 32;
const MAX_REPAIRS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NormalizedRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl NormalizedRegion {
    pub fn validate(&self) -> Result<(), String> {
        let values = [self.x, self.y, self.width, self.height];
        if values.iter().any(|value| !value.is_finite()) {
            return Err("region contains a non-finite coordinate".to_string());
        }
        if self.x < 0.0
            || self.y < 0.0
            || self.width <= 0.0
            || self.height <= 0.0
            || self.x + self.width > 1.0
            || self.y + self.height > 1.0
        {
            return Err("region must fit inside normalized [0,1] bounds".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtPlan {
    pub summary: String,
    pub reference_prompt: String,
    #[serde(default)]
    pub palette: Vec<u8>,
    #[serde(default)]
    pub protected_regions: Vec<NormalizedRegion>,
}

impl ArtPlan {
    pub fn validate(&self) -> Result<(), String> {
        validate_text("summary", &self.summary)?;
        validate_text("reference_prompt", &self.reference_prompt)?;
        if self.palette.len() > 16 {
            return Err("art plan palette exceeds 16 colors".to_string());
        }
        if self.protected_regions.len() > MAX_REGIONS {
            return Err("too many protected regions".to_string());
        }
        for region in &self.protected_regions {
            region.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CritiqueScores {
    pub semantic_fidelity: f32,
    pub subject_readability: f32,
    pub composition: f32,
    pub palette_coherence: f32,
    pub contour_continuity: f32,
    pub petscii_authenticity: f32,
}

impl CritiqueScores {
    pub fn validate(&self) -> Result<(), String> {
        let values = [
            self.semantic_fidelity,
            self.subject_readability,
            self.composition,
            self.palette_coherence,
            self.contour_continuity,
            self.petscii_authenticity,
        ];
        if values
            .iter()
            .any(|score| !score.is_finite() || !(0.0..=100.0).contains(score))
        {
            return Err("critique scores must be finite values from 0 to 100".to_string());
        }
        Ok(())
    }

    pub fn mean(&self) -> f32 {
        (self.semantic_fidelity
            + self.subject_readability
            + self.composition
            + self.palette_coherence
            + self.contour_continuity
            + self.petscii_authenticity)
            / 6.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionCritique {
    pub region: NormalizedRegion,
    pub problem: String,
    pub severity: f32,
}

impl RegionCritique {
    fn validate(&self) -> Result<(), String> {
        self.region.validate()?;
        validate_text("region problem", &self.problem)?;
        if !self.severity.is_finite() || !(0.0..=1.0).contains(&self.severity) {
            return Err("region severity must be between 0 and 1".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RepairDirective {
    IncreaseContrast {
        region: NormalizedRegion,
        amount: f32,
    },
    SimplifyRegion {
        region: NormalizedRegion,
        strength: f32,
    },
    ProtectSilhouette {
        region: NormalizedRegion,
    },
    ReduceDensity {
        region: NormalizedRegion,
        target: f32,
    },
    ShiftCrop {
        dx: f32,
        dy: f32,
        scale: f32,
    },
    ChangePaletteRole {
        role: String,
        color: u8,
    },
    ReplaceCell {
        x: u32,
        y: u32,
        glyph: u8,
        fg: u8,
        bg: u8,
    },
}

impl RepairDirective {
    fn validate(
        &self,
        grid_width: u32,
        grid_height: u32,
        allowed_colors: &[u8],
    ) -> Result<(), String> {
        match self {
            Self::IncreaseContrast { region, amount } => {
                region.validate()?;
                if !amount.is_finite() || !(-50.0..=50.0).contains(amount) {
                    return Err("contrast repair amount must be between -50 and 50".to_string());
                }
            }
            Self::SimplifyRegion { region, strength } => {
                region.validate()?;
                validate_unit("simplify strength", *strength)?;
            }
            Self::ProtectSilhouette { region } => region.validate()?,
            Self::ReduceDensity { region, target } => {
                region.validate()?;
                validate_unit("density target", *target)?;
            }
            Self::ShiftCrop { dx, dy, scale } => {
                if !dx.is_finite()
                    || !dy.is_finite()
                    || !scale.is_finite()
                    || !(-0.25..=0.25).contains(dx)
                    || !(-0.25..=0.25).contains(dy)
                    || !(0.75..=1.25).contains(scale)
                {
                    return Err("crop repair exceeds bounded shift/scale".to_string());
                }
            }
            Self::ChangePaletteRole { role, color } => {
                validate_text("palette role", role)?;
                validate_color(*color, allowed_colors)?;
            }
            Self::ReplaceCell { x, y, fg, bg, .. } => {
                if *x >= grid_width || *y >= grid_height {
                    return Err("cell replacement is outside the output grid".to_string());
                }
                validate_color(*fg, allowed_colors)?;
                validate_color(*bg, allowed_colors)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Critique {
    /// Candidate index selected by the critic when multiple previews are supplied.
    #[serde(default)]
    pub selected_candidate: usize,
    pub scores: CritiqueScores,
    #[serde(default)]
    pub regions: Vec<RegionCritique>,
    #[serde(default)]
    pub repairs: Vec<RepairDirective>,
    pub explanation: String,
}

impl Critique {
    pub fn from_json(
        json: &str,
        grid_width: u32,
        grid_height: u32,
        allowed_colors: &[u8],
    ) -> Result<Self, String> {
        if json.len() > 256 * 1024 {
            return Err("critique response exceeds 256 KiB".to_string());
        }
        let critique: Self = serde_json::from_str(json)
            .map_err(|error| format!("invalid critique JSON: {error}"))?;
        critique.validate(grid_width, grid_height, allowed_colors)?;
        Ok(critique)
    }

    pub fn validate(
        &self,
        grid_width: u32,
        grid_height: u32,
        allowed_colors: &[u8],
    ) -> Result<(), String> {
        self.scores.validate()?;
        validate_text("explanation", &self.explanation)?;
        if self.regions.len() > MAX_REGIONS {
            return Err("critique contains too many regions".to_string());
        }
        if self.repairs.len() > MAX_REPAIRS {
            return Err("critique contains too many repair directives".to_string());
        }
        for region in &self.regions {
            region.validate()?;
        }
        for repair in &self.repairs {
            repair.validate(grid_width, grid_height, allowed_colors)?;
        }
        Ok(())
    }

    pub fn validate_candidate_count(&self, count: usize) -> Result<(), String> {
        if count == 0 || self.selected_candidate >= count {
            return Err("selected_candidate is outside the submitted candidate list".to_string());
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

fn validate_unit(name: &str, value: f32) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{name} must be between 0 and 1"));
    }
    Ok(())
}

fn validate_color(color: u8, allowed_colors: &[u8]) -> Result<(), String> {
    if !allowed_colors.contains(&color) {
        return Err(format!("color {color} is not in the configured palette"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json() -> String {
        r#"{
          "selected_candidate": 0,
          "scores": {
            "semantic_fidelity": 80,
            "subject_readability": 75,
            "composition": 70,
            "palette_coherence": 90,
            "contour_continuity": 68,
            "petscii_authenticity": 88
          },
          "regions": [{
            "region": {"x": 0.1, "y": 0.2, "width": 0.3, "height": 0.4},
            "problem": "subject silhouette is noisy",
            "severity": 0.7
          }],
          "repairs": [{
            "type": "simplify_region",
            "region": {"x": 0.1, "y": 0.2, "width": 0.3, "height": 0.4},
            "strength": 0.5
          }],
          "explanation": "Simplify the central subject."
        }"#
        .to_string()
    }

    #[test]
    fn valid_critique_parses() {
        let critique =
            Critique::from_json(&valid_json(), 40, 25, &(0..16).collect::<Vec<_>>()).unwrap();
        assert_eq!(critique.repairs.len(), 1);
        assert!(critique.scores.mean() > 70.0);
    }

    #[test]
    fn malformed_json_is_rejected() {
        assert!(Critique::from_json("not-json", 40, 25, &[0, 1]).is_err());
    }

    #[test]
    fn out_of_bounds_region_is_rejected() {
        let json = valid_json().replace("\"width\": 0.3", "\"width\": 0.95");
        assert!(Critique::from_json(&json, 40, 25, &(0..16).collect::<Vec<_>>()).is_err());
    }

    #[test]
    fn unavailable_color_is_rejected() {
        let json = valid_json().replace(
            "\"type\": \"simplify_region\",\n            \"region\": {\"x\": 0.1, \"y\": 0.2, \"width\": 0.3, \"height\": 0.4},\n            \"strength\": 0.5",
            "\"type\": \"replace_cell\", \"x\": 1, \"y\": 1, \"glyph\": 2, \"fg\": 99, \"bg\": 0"
        );
        assert!(Critique::from_json(&json, 40, 25, &(0..16).collect::<Vec<_>>()).is_err());
    }
}
