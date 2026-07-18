use crate::{
    convert_image, convert_image_dithered, convert_image_dithered_prior, convert_image_top1,
    perceptual_tone_score, render_grid, score_grid, ConversionConfig, CorpusPrior,
    EdgeGrammarReport, OptimizationWeights, ScoreBreakdown,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const SCORE_TIE_EPSILON: f64 = 1e-12;
/// Half-glyph block size for the eye-averaged tone metric.
const PERCEPTUAL_BLOCK: u32 = 4;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BenchmarkSuite {
    pub version: u32,
    pub grid: [u32; 2],
    pub prompts: Vec<BenchmarkCase>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BenchmarkCase {
    pub id: String,
    pub category: String,
    pub prompt: String,
    #[serde(default)]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub mode: u8,
    pub baseline_top_k: usize,
    pub candidate_top_k: usize,
    pub preview_scale: u32,
}

impl Default for BenchmarkOptions {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            mode: 2,
            baseline_top_k: 1,
            candidate_top_k: 16,
            preview_scale: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkWinner {
    Baseline,
    Candidate,
    Tie,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkCaseReport {
    pub id: String,
    pub category: String,
    pub prompt: String,
    pub reference_path: String,
    pub width: u32,
    pub height: u32,
    pub baseline_score: ScoreBreakdown,
    pub candidate_score: ScoreBreakdown,
    /// Positive means the candidate reduced the total loss.
    pub improvement: f64,
    pub relative_improvement: f64,
    pub winner: BenchmarkWinner,
    /// Eye-averaged tone distance (lower is better). On this suite it tracks blinded human
    /// preference roughly three times better than the per-pixel reconstruction score, so it is
    /// reported alongside as a second, perception-aligned winner.
    pub baseline_perceptual: f64,
    pub candidate_perceptual: f64,
    pub perceptual_winner: BenchmarkWinner,
    pub candidate_edge_grammar: EdgeGrammarReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub cases: usize,
    pub candidate_wins: usize,
    pub ties: usize,
    pub baseline_wins: usize,
    pub win_or_tie_rate: f64,
    pub mean_baseline_score: f64,
    pub mean_candidate_score: f64,
    pub mean_improvement: f64,
    pub mean_relative_improvement: f64,
    pub perceptual_candidate_wins: usize,
    pub perceptual_ties: usize,
    pub perceptual_baseline_wins: usize,
    pub perceptual_win_or_tie_rate: f64,
    pub mean_baseline_perceptual: f64,
    pub mean_candidate_perceptual: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub version: u32,
    pub options: BenchmarkOptions,
    pub summary: BenchmarkSummary,
    pub cases: Vec<BenchmarkCaseReport>,
}

pub fn run_benchmark(
    manifest_path: &Path,
    reference_dir: &Path,
    output_dir: &Path,
    options: BenchmarkOptions,
) -> Result<BenchmarkReport, String> {
    validate_options(options)?;
    let suite: BenchmarkSuite = serde_json::from_slice(
        &fs::read(manifest_path)
            .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?,
    )
    .map_err(|error| {
        format!(
            "invalid benchmark manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    if suite.prompts.is_empty() {
        return Err("benchmark manifest contains no cases".to_string());
    }
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("failed to create {}: {error}", output_dir.display()))?;

    let width = options.width.unwrap_or(suite.grid[0]);
    let height = options.height.unwrap_or(suite.grid[1]);
    if width == 0 || height == 0 {
        return Err("benchmark grid dimensions must be non-zero".to_string());
    }
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let mut cases = Vec::with_capacity(suite.prompts.len());
    for case in &suite.prompts {
        validate_case_id(&case.id)?;
        let reference_path = resolve_reference(case, manifest_dir, reference_dir)?;
        let reference = image::open(&reference_path).map_err(|error| {
            format!(
                "failed to open reference for '{}': {}: {error}",
                case.id,
                reference_path.display()
            )
        })?;
        let baseline_config = ConversionConfig {
            width,
            height,
            mode: options.mode,
            top_k: options.baseline_top_k,
            contrast: 0.0,
        };
        let candidate_config = ConversionConfig {
            top_k: options.candidate_top_k,
            ..baseline_config.clone()
        };
        let baseline = convert_image_top1(&reference, &baseline_config)
            .map_err(|error| format!("baseline conversion failed for '{}': {error}", case.id))?;
        let candidate = convert_image(&reference, &candidate_config)
            .map_err(|error| format!("candidate conversion failed for '{}': {error}", case.id))?;
        let weights = OptimizationWeights::default();
        let baseline_score = score_grid(&baseline.grid, &baseline.reference, weights)
            .map_err(|error| format!("baseline scoring failed for '{}': {error}", case.id))?;
        let candidate_score = score_grid(&candidate.grid, &candidate.reference, weights)
            .map_err(|error| format!("candidate scoring failed for '{}': {error}", case.id))?;
        let improvement = baseline_score.total - candidate_score.total;
        let relative_improvement = if baseline_score.total > f64::EPSILON {
            improvement / baseline_score.total
        } else {
            0.0
        };
        let winner = classify_winner(baseline_score.total, candidate_score.total);
        let baseline_perceptual =
            perceptual_tone_score(&baseline.grid, &baseline.reference, PERCEPTUAL_BLOCK)?;
        let candidate_perceptual =
            perceptual_tone_score(&candidate.grid, &candidate.reference, PERCEPTUAL_BLOCK)?;
        let perceptual_winner = classify_winner(baseline_perceptual, candidate_perceptual);
        let case_dir = output_dir.join(&case.id);
        fs::create_dir_all(&case_dir)
            .map_err(|error| format!("failed to create {}: {error}", case_dir.display()))?;
        reference
            .save(case_dir.join("reference.png"))
            .map_err(|error| format!("failed to save reference for '{}': {error}", case.id))?;
        save_grid_artifacts(&case_dir, "baseline", &baseline.grid, options.preview_scale)?;
        save_grid_artifacts(
            &case_dir,
            "candidate",
            &candidate.grid,
            options.preview_scale,
        )?;
        let case_report = BenchmarkCaseReport {
            id: case.id.clone(),
            category: case.category.clone(),
            prompt: case.prompt.clone(),
            reference_path: report_reference_path(case, &reference_path),
            width,
            height,
            baseline_score,
            candidate_score,
            improvement,
            relative_improvement,
            winner,
            baseline_perceptual,
            candidate_perceptual,
            perceptual_winner,
            candidate_edge_grammar: candidate.edge_grammar,
        };
        save_json(&case_dir.join("metrics.json"), &case_report)?;
        cases.push(case_report);
    }

    let report = BenchmarkReport {
        version: suite.version,
        options,
        summary: summarize(&cases),
        cases,
    };
    save_json(&output_dir.join("report.json"), &report)?;
    Ok(report)
}

/// One reference's candidate-vs-dither comparison under both the per-pixel reconstruction score
/// and the eye-averaged perceptual tone score. Lower scores are better for both.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DitherEvalRow {
    pub id: String,
    pub category: String,
    /// Three arms scored under each metric: `baseline` is the top-1 matcher, `plain` is the
    /// current full pipeline, `dither` is that pipeline with selective dithering. Lower is better
    /// for every metric. Comparing baseline vs plain tests which metric tracks human preference;
    /// comparing plain vs dither tests whether dithering helps.
    pub reconstruction_baseline: f64,
    pub reconstruction_plain: f64,
    pub reconstruction_dither: f64,
    pub perceptual_baseline: f64,
    pub perceptual_plain: f64,
    pub perceptual_dither: f64,
    /// Dithered cells in the dithered result (glyph differs from a plain solid/space fill).
    pub dither_cells: usize,
    /// Corpus-derived "human-likeness" bigram NLL (lower is more human-like), when a prior is
    /// supplied. Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub naturalness_baseline: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub naturalness_plain: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub naturalness_dither: Option<f64>,
}

/// Measure selective dithering against the current pipeline on a reference suite. This is a
/// standalone diagnostic: it does not touch the versioned benchmark report or its snapshot. For
/// each case it converts with and without dithering at identical settings, then reports both the
/// per-pixel reconstruction score (which structurally penalizes dithering) and the perceptual
/// tone score (which credits correct eye-averaged tone).
pub fn run_dither_eval(
    manifest_path: &Path,
    reference_dir: &Path,
    output_dir: &Path,
    options: BenchmarkOptions,
    prior: Option<&CorpusPrior>,
) -> Result<Vec<DitherEvalRow>, String> {
    validate_options(options)?;
    let suite: BenchmarkSuite = serde_json::from_slice(
        &fs::read(manifest_path)
            .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?,
    )
    .map_err(|error| format!("invalid benchmark manifest {}: {error}", manifest_path.display()))?;
    if suite.prompts.is_empty() {
        return Err("benchmark manifest contains no cases".to_string());
    }
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("failed to create {}: {error}", output_dir.display()))?;
    let width = options.width.unwrap_or(suite.grid[0]);
    let height = options.height.unwrap_or(suite.grid[1]);
    if width == 0 || height == 0 {
        return Err("benchmark grid dimensions must be non-zero".to_string());
    }
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let weights = OptimizationWeights::default();
    let mut rows = Vec::with_capacity(suite.prompts.len());
    for case in &suite.prompts {
        validate_case_id(&case.id)?;
        let reference_path = resolve_reference(case, manifest_dir, reference_dir)?;
        let reference = image::open(&reference_path)
            .map_err(|error| format!("failed to open reference for '{}': {error}", case.id))?;
        let config = ConversionConfig {
            width,
            height,
            mode: options.mode,
            top_k: options.candidate_top_k,
            contrast: 0.0,
        };
        let baseline = convert_image_top1(&reference, &config)
            .map_err(|error| format!("baseline conversion failed for '{}': {error}", case.id))?;
        let plain = convert_image(&reference, &config)
            .map_err(|error| format!("plain conversion failed for '{}': {error}", case.id))?;
        // When a prior is supplied, the dither arm is the corpus-regularized variant so the
        // comparison reflects the tone-vs-layout trade the regularizer makes.
        let dither = match prior {
            Some(prior) => convert_image_dithered_prior(&reference, &config, prior),
            None => convert_image_dithered(&reference, &config),
        }
        .map_err(|error| format!("dither conversion failed for '{}': {error}", case.id))?;
        let dither_cells = plain
            .grid
            .cells
            .iter()
            .zip(&dither.grid.cells)
            .filter(|(a, b)| a.glyph != b.glyph)
            .count();
        let row = DitherEvalRow {
            id: case.id.clone(),
            category: case.category.clone(),
            reconstruction_baseline: score_grid(&baseline.grid, &baseline.reference, weights)?.total,
            reconstruction_plain: score_grid(&plain.grid, &plain.reference, weights)?.total,
            reconstruction_dither: score_grid(&dither.grid, &dither.reference, weights)?.total,
            perceptual_baseline: perceptual_tone_score(
                &baseline.grid,
                &baseline.reference,
                PERCEPTUAL_BLOCK,
            )?,
            perceptual_plain: perceptual_tone_score(&plain.grid, &plain.reference, PERCEPTUAL_BLOCK)?,
            perceptual_dither: perceptual_tone_score(
                &dither.grid,
                &dither.reference,
                PERCEPTUAL_BLOCK,
            )?,
            dither_cells,
            naturalness_baseline: prior.map(|prior| prior.naturalness(&baseline.grid).bigram_nll),
            naturalness_plain: prior.map(|prior| prior.naturalness(&plain.grid).bigram_nll),
            naturalness_dither: prior.map(|prior| prior.naturalness(&dither.grid).bigram_nll),
        };
        let case_dir = output_dir.join(&case.id);
        fs::create_dir_all(&case_dir)
            .map_err(|error| format!("failed to create {}: {error}", case_dir.display()))?;
        save_grid_artifacts(&case_dir, "plain", &plain.grid, options.preview_scale)?;
        save_grid_artifacts(&case_dir, "dither", &dither.grid, options.preview_scale)?;
        save_json(&case_dir.join("dither-eval.json"), &row)?;
        rows.push(row);
    }
    save_json(&output_dir.join("dither-eval.json"), &rows)?;
    Ok(rows)
}

fn validate_options(options: BenchmarkOptions) -> Result<(), String> {
    if options.mode > 2 {
        return Err(format!("benchmark mode {} is unsupported", options.mode));
    }
    if options.baseline_top_k != 1 {
        return Err("benchmark baseline top-k is fixed at 1".to_string());
    }
    if options.candidate_top_k == 0 {
        return Err("benchmark top-k values must be non-zero".to_string());
    }
    if options.candidate_top_k < options.baseline_top_k {
        return Err("candidate top-k must be at least baseline top-k".to_string());
    }
    if options.preview_scale == 0 {
        return Err("benchmark preview scale must be non-zero".to_string());
    }
    Ok(())
}

fn report_reference_path(case: &BenchmarkCase, resolved: &Path) -> String {
    match case.reference.as_deref().map(Path::new) {
        Some(path) if !path.is_absolute() => path.to_string_lossy().into_owned(),
        _ => resolved
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    }
}

fn validate_case_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(format!("invalid benchmark case id '{id}'"));
    }
    Ok(())
}

fn resolve_reference(
    case: &BenchmarkCase,
    manifest_dir: &Path,
    reference_dir: &Path,
) -> Result<PathBuf, String> {
    if let Some(reference) = &case.reference {
        let path = PathBuf::from(reference);
        let path = if path.is_absolute() {
            path
        } else {
            manifest_dir.join(path)
        };
        return path
            .is_file()
            .then_some(path)
            .ok_or_else(|| format!("reference for '{}' does not exist", case.id));
    }
    for extension in ["png", "jpg", "jpeg", "webp"] {
        let path = reference_dir.join(format!("{}.{}", case.id, extension));
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(format!(
        "reference for '{}' was not found in {}",
        case.id,
        reference_dir.display()
    ))
}

fn save_grid_artifacts(
    output_dir: &Path,
    stem: &str,
    grid: &crate::PetsciiGrid,
    preview_scale: u32,
) -> Result<(), String> {
    fs::write(output_dir.join(format!("{stem}.pix")), grid.to_pix_string())
        .map_err(|error| format!("failed to save {stem}.pix: {error}"))?;
    render_grid(grid, preview_scale)?
        .save(output_dir.join(format!("{stem}.png")))
        .map_err(|error| format!("failed to save {stem}.png: {error}"))
}

fn save_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut encoded = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("failed to encode {}: {error}", path.display()))?;
    encoded.push(b'\n');
    fs::write(path, encoded).map_err(|error| format!("failed to save {}: {error}", path.display()))
}

fn classify_winner(baseline: f64, candidate: f64) -> BenchmarkWinner {
    if candidate + SCORE_TIE_EPSILON < baseline {
        BenchmarkWinner::Candidate
    } else if baseline + SCORE_TIE_EPSILON < candidate {
        BenchmarkWinner::Baseline
    } else {
        BenchmarkWinner::Tie
    }
}

fn summarize(cases: &[BenchmarkCaseReport]) -> BenchmarkSummary {
    let candidate_wins = cases
        .iter()
        .filter(|case| case.winner == BenchmarkWinner::Candidate)
        .count();
    let ties = cases
        .iter()
        .filter(|case| case.winner == BenchmarkWinner::Tie)
        .count();
    let baseline_wins = cases.len() - candidate_wins - ties;
    let perceptual_candidate_wins = cases
        .iter()
        .filter(|case| case.perceptual_winner == BenchmarkWinner::Candidate)
        .count();
    let perceptual_ties = cases
        .iter()
        .filter(|case| case.perceptual_winner == BenchmarkWinner::Tie)
        .count();
    let perceptual_baseline_wins = cases.len() - perceptual_candidate_wins - perceptual_ties;
    let denominator = cases.len().max(1) as f64;
    BenchmarkSummary {
        cases: cases.len(),
        candidate_wins,
        ties,
        baseline_wins,
        win_or_tie_rate: (candidate_wins + ties) as f64 / denominator,
        mean_baseline_score: cases
            .iter()
            .map(|case| case.baseline_score.total)
            .sum::<f64>()
            / denominator,
        mean_candidate_score: cases
            .iter()
            .map(|case| case.candidate_score.total)
            .sum::<f64>()
            / denominator,
        mean_improvement: cases.iter().map(|case| case.improvement).sum::<f64>() / denominator,
        mean_relative_improvement: cases
            .iter()
            .map(|case| case.relative_improvement)
            .sum::<f64>()
            / denominator,
        perceptual_candidate_wins,
        perceptual_ties,
        perceptual_baseline_wins,
        perceptual_win_or_tie_rate: (perceptual_candidate_wins + perceptual_ties) as f64
            / denominator,
        mean_baseline_perceptual: cases
            .iter()
            .map(|case| case.baseline_perceptual)
            .sum::<f64>()
            / denominator,
        mean_candidate_perceptual: cases
            .iter()
            .map(|case| case.candidate_perceptual)
            .sum::<f64>()
            / denominator,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use serde_json::Value;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SNAPSHOT_FLOAT_TOLERANCE: f64 = 1e-12;

    fn temporary_directory(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("petii-{name}-{}-{nonce}", std::process::id()))
    }

    fn assert_json_close(actual: &Value, expected: &Value, path: &str) {
        match (actual, expected) {
            (Value::Number(actual), Value::Number(expected)) => {
                let actual = actual.as_f64().unwrap();
                let expected = expected.as_f64().unwrap();
                let scale = actual.abs().max(expected.abs()).max(1.0);
                assert!(
                    (actual - expected).abs() <= SNAPSHOT_FLOAT_TOLERANCE * scale,
                    "numeric snapshot mismatch at {path}: actual={actual}, expected={expected}"
                );
            }
            (Value::Array(actual), Value::Array(expected)) => {
                assert_eq!(actual.len(), expected.len(), "array length at {path}");
                for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                    assert_json_close(actual, expected, &format!("{path}[{index}]"));
                }
            }
            (Value::Object(actual), Value::Object(expected)) => {
                assert_eq!(
                    actual.keys().collect::<Vec<_>>(),
                    expected.keys().collect::<Vec<_>>(),
                    "object keys at {path}"
                );
                for (key, expected) in expected {
                    assert_json_close(&actual[key], expected, &format!("{path}.{key}"));
                }
            }
            _ => assert_eq!(actual, expected, "snapshot mismatch at {path}"),
        }
    }

    #[test]
    fn winner_classification_is_deterministic() {
        assert_eq!(classify_winner(0.2, 0.1), BenchmarkWinner::Candidate);
        assert_eq!(classify_winner(0.1, 0.2), BenchmarkWinner::Baseline);
        assert_eq!(classify_winner(0.1, 0.1), BenchmarkWinner::Tie);
    }

    #[test]
    fn runner_writes_baseline_candidate_and_summary_artifacts() {
        let root = temporary_directory("benchmark");
        let references = root.join("references");
        let output = root.join("output");
        fs::create_dir_all(&references).unwrap();
        RgbImage::from_pixel(16, 16, Rgb([50, 90, 150]))
            .save(references.join("sample.png"))
            .unwrap();
        let manifest = root.join("suite.json");
        fs::write(
            &manifest,
            br#"{"version":1,"grid":[2,2],"prompts":[{"id":"sample","category":"smoke","prompt":"sample"}]}"#,
        )
        .unwrap();

        let report = run_benchmark(
            &manifest,
            &references,
            &output,
            BenchmarkOptions {
                candidate_top_k: 2,
                ..BenchmarkOptions::default()
            },
        )
        .unwrap();

        assert_eq!(report.summary.cases, 1);
        assert_eq!(report.cases[0].reference_path, "sample.png");
        for path in [
            "report.json",
            "sample/reference.png",
            "sample/baseline.pix",
            "sample/baseline.png",
            "sample/candidate.pix",
            "sample/candidate.png",
            "sample/metrics.json",
        ] {
            assert!(output.join(path).is_file(), "missing {path}");
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "versioned reference suite; run explicitly in release CI"]
    fn recorded_benchmark_v1_matches_snapshot() {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let suite_dir = crate_dir.join("benchmark/v1");
        let output = temporary_directory("benchmark-v1-snapshot");
        let expected: BenchmarkReport =
            serde_json::from_slice(&fs::read(suite_dir.join("expected-report.json")).unwrap())
                .unwrap();

        let actual = run_benchmark(
            &suite_dir.join("prompts.json"),
            &suite_dir.join("references"),
            &output,
            BenchmarkOptions::default(),
        )
        .unwrap();
        fs::remove_dir_all(output).unwrap();

        assert_json_close(
            &serde_json::to_value(actual).unwrap(),
            &serde_json::to_value(expected).unwrap(),
            "$",
        );
    }
}
