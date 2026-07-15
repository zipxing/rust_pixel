use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateArtifact {
    pub id: String,
    pub pix_path: String,
    pub preview_path: String,
    pub deterministic_score: f64,
    pub critic_score: Option<f32>,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedResponse {
    pub kind: String,
    pub model: String,
    pub request_hash: String,
    pub body: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunManifest {
    pub version: u32,
    pub prompt: String,
    pub seed: u64,
    pub width: u32,
    pub height: u32,
    pub palette: Vec<u8>,
    pub conversion: crate::ConversionConfig,
    pub max_iterations: usize,
    pub max_candidates: usize,
    pub candidates: Vec<CandidateArtifact>,
    pub responses: Vec<RecordedResponse>,
}

impl RunManifest {
    pub fn save_redacted(&self, path: &Path) -> Result<(), String> {
        let mut value = serde_json::to_value(self)
            .map_err(|error| format!("failed to serialize run manifest: {error}"))?;
        redact_value(&mut value);
        let bytes = serde_json::to_vec_pretty(&value)
            .map_err(|error| format!("failed to encode run manifest: {error}"))?;
        std::fs::write(path, bytes)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid run manifest {}: {error}", path.display()))
    }
}

fn redact_value(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for (key, child) in object.iter_mut() {
                let normalized = key.to_ascii_lowercase();
                if normalized.contains("api_key")
                    || normalized.contains("authorization")
                    || normalized == "token"
                    || normalized.ends_with("_token")
                {
                    *child = Value::String("[REDACTED]".to_string());
                } else {
                    redact_value(child);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_value(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trip_redacts_secrets() {
        let path = std::env::temp_dir().join(format!("petii-manifest-{}.json", std::process::id()));
        let manifest = RunManifest {
            version: 1,
            prompt: "moon witch".to_string(),
            seed: 7,
            width: 40,
            height: 25,
            palette: (0..16).collect(),
            conversion: crate::ConversionConfig::default(),
            max_iterations: 4,
            max_candidates: 4,
            candidates: vec![],
            responses: vec![RecordedResponse {
                kind: "critic".to_string(),
                model: "test".to_string(),
                request_hash: "abc".to_string(),
                body: serde_json::json!({"authorization": "Bearer secret", "ok": true}),
            }],
        };
        manifest.save_redacted(&path).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(!saved.contains("Bearer secret"));
        let loaded = RunManifest::load(&path).unwrap();
        assert_eq!(loaded.responses[0].body["authorization"], "[REDACTED]");
        let _ = std::fs::remove_file(path);
    }
}
