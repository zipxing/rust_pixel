use image::{DynamicImage, RgbaImage};
use petii::{
    ai::{
        run_with_reference, AiLoopBudget, ArtPlan, CandidateArtifact, Critique, CritiqueScores,
        MultimodalCritic, OpenAiCompatibleProvider, ReferenceGenerator, RunManifest,
    },
    ConversionConfig,
};
use serde_json::json;
use std::{fs, path::PathBuf, time::SystemTime};

pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(());
    }
    let prompt = args[0].trim();
    if prompt.is_empty() || prompt.starts_with("--") {
        return Err("AI mode requires a quoted prompt as its first argument".to_string());
    }

    let width = parse_value(args, "--width", 40_u32)?;
    let explicit_height = parse_optional_value(args, "--height")?;
    let top_k = parse_value(args, "--top-k", 6_usize)?;
    let max_iterations = parse_value(args, "--iterations", 4_usize)?;
    let max_candidates = parse_value(args, "--candidates", 4_usize)?;
    let seed = parse_value(args, "--seed", 0_u64)?;
    let offline = has_flag(args, "--offline");
    let input = value_after(args, "--input");
    if offline && input.is_none() {
        return Err(
            "--offline requires --input because it cannot generate a reference".to_string(),
        );
    }

    let budget = AiLoopBudget {
        max_iterations,
        max_candidates,
        preview_scale: 2,
        allowed_colors: (0..16).collect(),
    };

    let (plan, reference, config, result) = if offline {
        let reference = open_input(input.as_deref().unwrap())?;
        let plan = input_plan(prompt);
        let config = aspect_preserving_config(&reference, width, explicit_height, top_k)?;
        let result = run_with_reference(prompt, &reference, &config, &OfflineCritic, &budget)?;
        (plan, reference, config, result)
    } else {
        let provider = OpenAiCompatibleProvider::from_env()?;
        let (plan, reference) = match input {
            Some(path) => (input_plan(prompt), open_input(&path)?),
            None => provider.generate_reference(prompt, width, explicit_height.unwrap_or(width))?,
        };
        let config = aspect_preserving_config(&reference, width, explicit_height, top_k)?;
        let result = run_with_reference(prompt, &reference, &config, &provider, &budget)?;
        (plan, reference, config, result)
    };

    let output_dir = value_after(args, "--output-dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_output_dir);
    save_run(
        &output_dir,
        prompt,
        seed,
        &plan,
        &reference,
        &result,
        &config,
        &budget,
    )?;
    eprintln!(
        "PETSCII AI run complete: {} (grid={}x{}, score={:.6}, iterations={})",
        output_dir.display(),
        config.width,
        config.height,
        result.deterministic_score.total,
        result.iterations
    );
    for warning in &result.warnings {
        eprintln!("Warning: {warning}");
    }
    Ok(())
}

fn print_usage() {
    println!("EXPERIMENTAL AI MODE:");
    println!("  petii ai \"PROMPT\" [--input IMAGE] [--offline]");
    println!("        [--width 40] [--height ROWS] [--top-k 6]");
    println!("        [--iterations 4] [--candidates 4] [--seed 0]");
    println!("        [--output-dir DIRECTORY]");
    println!();
    println!("Live mode reads PETII_AI_API_KEY and optional PETII_AI_API_BASE,");
    println!("PETII_AI_IMAGE_MODEL, and PETII_AI_VISION_MODEL.");
    println!("Without --height, rows are derived from the reference-image aspect ratio.");
    println!("Offline mode requires --input and runs only the deterministic pipeline.");
}

fn value_after(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn parse_value<T>(args: &[String], flag: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match value_after(args, flag) {
        Some(raw) => raw
            .parse()
            .map_err(|error| format!("invalid {flag} value '{raw}': {error}")),
        None => Ok(default),
    }
}

fn parse_optional_value<T>(args: &[String], flag: &str) -> Result<Option<T>, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value_after(args, flag)
        .map(|raw| {
            raw.parse()
                .map_err(|error| format!("invalid {flag} value '{raw}': {error}"))
        })
        .transpose()
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn open_input(path: &str) -> Result<DynamicImage, String> {
    image::open(path).map_err(|error| format!("failed to open input image '{path}': {error}"))
}

fn aspect_preserving_config(
    reference: &DynamicImage,
    width: u32,
    explicit_height: Option<u32>,
    top_k: usize,
) -> Result<ConversionConfig, String> {
    if reference.width() == 0 || reference.height() == 0 {
        return Err("reference image dimensions must be non-zero".to_string());
    }
    let height = explicit_height.unwrap_or_else(|| {
        (width as f64 * reference.height() as f64 / reference.width() as f64)
            .round()
            .max(1.0) as u32
    });
    let config = ConversionConfig {
        width,
        height,
        mode: 1,
        top_k,
        contrast: 0.0,
    };
    config.validate()?;
    Ok(config)
}

fn input_plan(prompt: &str) -> ArtPlan {
    ArtPlan {
        summary: prompt.to_string(),
        reference_prompt: "User-supplied reference image".to_string(),
        palette: (0..16).collect(),
        protected_regions: Vec::new(),
    }
}

fn default_output_dir() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    PathBuf::from(format!("tmp/petii-ai/run-{timestamp}"))
}

#[allow(clippy::too_many_arguments)]
fn save_run(
    output_dir: &PathBuf,
    prompt: &str,
    seed: u64,
    plan: &ArtPlan,
    reference: &DynamicImage,
    result: &petii::ai::AiLoopResult,
    config: &ConversionConfig,
    budget: &AiLoopBudget,
) -> Result<(), String> {
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("failed to create {}: {error}", output_dir.display()))?;
    reference
        .save(output_dir.join("reference.png"))
        .map_err(|error| format!("failed to save reference: {error}"))?;
    fs::write(output_dir.join("final.pix"), result.grid.to_pix_string())
        .map_err(|error| format!("failed to save final.pix: {error}"))?;
    result
        .candidates
        .last()
        .ok_or_else(|| "AI loop produced no candidates".to_string())?
        .preview
        .save(output_dir.join("final.png"))
        .map_err(|error| format!("failed to save final.png: {error}"))?;

    let mut artifacts = Vec::new();
    for (index, candidate) in result.candidates.iter().enumerate() {
        let id = if index + 1 == result.candidates.len() {
            "final".to_string()
        } else {
            format!("candidate-{index:02}")
        };
        let pix_name = format!("{id}.pix");
        let preview_name = format!("{id}.png");
        fs::write(output_dir.join(&pix_name), candidate.grid.to_pix_string())
            .map_err(|error| format!("failed to save {pix_name}: {error}"))?;
        candidate
            .preview
            .save(output_dir.join(&preview_name))
            .map_err(|error| format!("failed to save {preview_name}: {error}"))?;
        artifacts.push(CandidateArtifact {
            id,
            pix_path: pix_name,
            preview_path: preview_name,
            deterministic_score: candidate.deterministic_score.total,
            critic_score: candidate.selected.then(|| result.critic.scores.mean()),
            selected: candidate.selected,
        });
    }
    save_gallery(output_dir, result)?;

    let summary = json!({
        "art_plan": plan,
        "critique": result.critic,
        "deterministic_score": result.deterministic_score,
        "iterations": result.iterations,
        "warnings": result.warnings,
    });
    fs::write(
        output_dir.join("critique.json"),
        serde_json::to_vec_pretty(&summary).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to save critique.json: {error}"))?;

    RunManifest {
        version: 1,
        prompt: prompt.to_string(),
        seed,
        width: config.width,
        height: config.height,
        palette: (0..16).collect(),
        conversion: config.clone(),
        max_iterations: budget.max_iterations,
        max_candidates: budget.max_candidates,
        candidates: artifacts,
        responses: Vec::new(),
    }
    .save_redacted(&output_dir.join("manifest.json"))
}

fn save_gallery(output_dir: &PathBuf, result: &petii::ai::AiLoopResult) -> Result<(), String> {
    let first = result
        .candidates
        .first()
        .ok_or_else(|| "AI loop produced no candidate previews".to_string())?;
    let cell_width = first.preview.width();
    let cell_height = first.preview.height();
    let mut gallery = RgbaImage::new(cell_width * result.candidates.len() as u32, cell_height);
    for (index, candidate) in result.candidates.iter().enumerate() {
        image::imageops::overlay(
            &mut gallery,
            &candidate.preview.to_rgba8(),
            (index as u32 * cell_width) as i64,
            0,
        );
    }
    gallery
        .save(output_dir.join("gallery.png"))
        .map_err(|error| format!("failed to save gallery.png: {error}"))
}

struct OfflineCritic;

impl MultimodalCritic for OfflineCritic {
    fn critique(
        &self,
        _prompt: &str,
        _reference: &DynamicImage,
        _candidates: &[DynamicImage],
        _grid_width: u32,
        _grid_height: u32,
        _allowed_colors: &[u8],
    ) -> Result<Critique, String> {
        Ok(Critique {
            selected_candidate: 0,
            scores: CritiqueScores {
                semantic_fidelity: 0.0,
                subject_readability: 0.0,
                composition: 0.0,
                palette_coherence: 0.0,
                contour_continuity: 0.0,
                petscii_authenticity: 0.0,
            },
            regions: Vec::new(),
            repairs: Vec::new(),
            explanation: "Offline deterministic selection; no AI critic was called.".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_reference_produces_square_grid() {
        let reference = DynamicImage::new_rgba8(1024, 1024);
        let config = aspect_preserving_config(&reference, 40, None, 4).unwrap();
        assert_eq!((config.width, config.height), (40, 40));
    }

    #[test]
    fn landscape_and_portrait_references_preserve_aspect() {
        let landscape = DynamicImage::new_rgba8(1600, 900);
        let portrait = DynamicImage::new_rgba8(800, 1200);
        assert_eq!(
            aspect_preserving_config(&landscape, 40, None, 4)
                .unwrap()
                .height,
            23
        );
        assert_eq!(
            aspect_preserving_config(&portrait, 40, None, 4)
                .unwrap()
                .height,
            60
        );
    }

    #[test]
    fn explicit_height_overrides_reference_aspect() {
        let reference = DynamicImage::new_rgba8(1024, 1024);
        let config = aspect_preserving_config(&reference, 40, Some(25), 4).unwrap();
        assert_eq!((config.width, config.height), (40, 25));
    }
}
