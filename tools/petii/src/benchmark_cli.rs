use petii::{run_benchmark, run_dither_eval, BenchmarkOptions, CorpusPrior};
use std::path::{Path, PathBuf};

pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty()
        || args
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
    {
        print_usage();
        return Ok(());
    }
    let manifest = PathBuf::from(&args[0]);
    if !manifest.is_file() {
        return Err(format!(
            "benchmark manifest '{}' does not exist",
            manifest.display()
        ));
    }
    let default_reference_dir = manifest
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("references");
    let reference_dir = value_after(args, "--reference-dir")
        .map(PathBuf::from)
        .unwrap_or(default_reference_dir);
    let output_dir = value_after(args, "--output-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tmp/petii-benchmark"));
    let options = BenchmarkOptions {
        width: parse_optional_value(args, "--width")?,
        height: parse_optional_value(args, "--height")?,
        mode: parse_value(args, "--mode", 2_u8)?,
        baseline_top_k: parse_value(args, "--baseline-top-k", 1_usize)?,
        candidate_top_k: parse_value(args, "--candidate-top-k", 16_usize)?,
        preview_scale: parse_value(args, "--preview-scale", 2_u32)?,
    };
    if args.iter().any(|argument| argument == "--dither-eval") {
        let prior_path = value_after(args, "--corpus-prior");
        return run_dither_evaluation(
            &manifest,
            &reference_dir,
            &output_dir,
            options,
            prior_path.as_deref(),
        );
    }
    let report = run_benchmark(&manifest, &reference_dir, &output_dir, options)?;
    eprintln!(
        "PETSCII benchmark complete: {} cases, report={}",
        report.summary.cases,
        output_dir.join("report.json").display()
    );
    eprintln!(
        "  reconstruction: candidate wins/ties/losses={}/{}/{}, win-or-tie={:.0}%, mean improvement={:.2}%",
        report.summary.candidate_wins,
        report.summary.ties,
        report.summary.baseline_wins,
        report.summary.win_or_tie_rate * 100.0,
        report.summary.mean_relative_improvement * 100.0,
    );
    eprintln!(
        "  perceptual:     candidate wins/ties/losses={}/{}/{}, win-or-tie={:.0}%, mean tone {:.3} -> {:.3}",
        report.summary.perceptual_candidate_wins,
        report.summary.perceptual_ties,
        report.summary.perceptual_baseline_wins,
        report.summary.perceptual_win_or_tie_rate * 100.0,
        report.summary.mean_baseline_perceptual,
        report.summary.mean_candidate_perceptual,
    );
    Ok(())
}

fn run_dither_evaluation(
    manifest: &Path,
    reference_dir: &Path,
    output_dir: &Path,
    options: BenchmarkOptions,
    prior_path: Option<&str>,
) -> Result<(), String> {
    let prior = prior_path.map(|path| CorpusPrior::load(Path::new(path))).transpose()?;
    let rows = run_dither_eval(manifest, reference_dir, output_dir, options, prior.as_ref())?;
    eprintln!(
        "{:<20} {:>8} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "case", "cells", "recon+", "recon.d", "percep+", "percep.d", "human+", "human.d"
    );
    let mut perceptual_gains = 0usize;
    for row in &rows {
        if row.perceptual_dither < row.perceptual_plain {
            perceptual_gains += 1;
        }
        eprintln!(
            "{:<20} {:>8} {:>10.5} {:>10.5} {:>10.4} {:>10.4} {:>10} {:>10}",
            row.id,
            row.dither_cells,
            row.reconstruction_plain,
            row.reconstruction_dither,
            row.perceptual_plain,
            row.perceptual_dither,
            row.naturalness_plain
                .map_or_else(|| "-".to_string(), |value| format!("{value:.4}")),
            row.naturalness_dither
                .map_or_else(|| "-".to_string(), |value| format!("{value:.4}")),
        );
    }
    let mean = |select: fn(&petii::DitherEvalRow) -> f64| -> f64 {
        rows.iter().map(select).sum::<f64>() / rows.len().max(1) as f64
    };
    eprintln!(
        "dither-eval: {} cases, dither improves perceptual tone in {}/{}, mean perceptual {:.4} -> {:.4}, results={}",
        rows.len(),
        perceptual_gains,
        rows.len(),
        mean(|row| row.perceptual_plain),
        mean(|row| row.perceptual_dither),
        output_dir.join("dither-eval.json").display()
    );
    if prior.is_some() {
        eprintln!(
            "human-likeness (bigram NLL, lower is more human-like): mean {:.4} -> {:.4}",
            mean(|row| row.naturalness_plain.unwrap_or(0.0)),
            mean(|row| row.naturalness_dither.unwrap_or(0.0)),
        );
    }
    Ok(())
}

fn print_usage() {
    println!("PETSCII BENCHMARK:");
    println!("  petii benchmark MANIFEST.json [--reference-dir DIRECTORY]");
    println!("        [--output-dir DIRECTORY] [--width N] [--height N]");
    println!("        [--mode 0|1|2] [--baseline-top-k 1] [--candidate-top-k 16]");
    println!("        [--preview-scale 2]");
    println!();
    println!("Each case may declare a reference path in the manifest. Otherwise the runner");
    println!("looks for <reference-dir>/<case-id>.png|jpg|jpeg|webp.");
}

fn value_after(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|argument| argument == flag)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_optional_and_default_values() {
        let args = vec![
            "suite.json".to_string(),
            "--width".to_string(),
            "60".to_string(),
        ];
        assert_eq!(
            parse_optional_value::<u32>(&args, "--width").unwrap(),
            Some(60)
        );
        assert_eq!(parse_value(&args, "--mode", 2_u8).unwrap(), 2);
    }
}
