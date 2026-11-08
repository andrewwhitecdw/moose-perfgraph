//! Command-line entry point for `moose-perfgraph`.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use moose_perfgraph::parse::{SectionRow, parse};

#[derive(Parser)]
#[command(
    name = "moose-perfgraph",
    about = "Rank MOOSE PerfGraph sections by time",
    version
)]
struct Cli {
    /// MOOSE console output (or any file containing PerfGraph tables)
    file: PathBuf,

    /// Show at most N sections per ranking (0 = all)
    #[arg(long, default_value_t = 20)]
    top: usize,

    /// Only show sections whose name contains this substring (case-insensitive)
    #[arg(long, value_name = "TEXT")]
    name: Option<String>,

    /// Only show sections with at least this many seconds of self time
    #[arg(long, value_name = "SECONDS", default_value_t = 0.0)]
    min_self: f64,
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();
    let text = std::fs::read_to_string(&cli.file)
        .with_context(|| format!("reading {}", cli.file.display()))?;
    let data = parse(&text);

    if data.graph.is_empty() && data.heaviest_sections.is_empty() {
        eprintln!("no PerfGraph sections found in {}", cli.file.display());
        return Ok(ExitCode::FAILURE);
    }

    if !data.graph.is_empty() {
        println!("performance graph: {} sections", data.graph.len());
        println!();
        println!("top by self time:");
        print_ranking(&data.graph, Ranking::SelfTime, &cli);
        println!();
        println!("top by total time:");
        print_ranking(&data.graph, Ranking::TotalTime, &cli);
    }
    if !data.heaviest_sections.is_empty() {
        if !data.graph.is_empty() {
            println!();
        }
        println!("heaviest sections (as reported by MOOSE):");
        print_ranking(&data.heaviest_sections, Ranking::SelfTime, &cli);
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(Clone, Copy)]
enum Ranking {
    SelfTime,
    TotalTime,
}

fn print_ranking(rows: &[SectionRow], by: Ranking, cli: &Cli) {
    let mut rows: Vec<&SectionRow> = rows
        .iter()
        .filter(|r| r.self_time >= cli.min_self)
        .filter(|r| {
            cli.name
                .as_ref()
                .is_none_or(|n| r.name.to_lowercase().contains(&n.to_lowercase()))
        })
        .collect();
    rows.sort_by(|a, b| {
        let key = |r: &&SectionRow| match by {
            Ranking::SelfTime => r.self_time,
            Ranking::TotalTime => r.total_time,
        };
        key(b)
            .partial_cmp(&key(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    let limit = if cli.top == 0 { rows.len() } else { cli.top };
    if rows.is_empty() {
        println!("  (no sections match the filters)");
        return;
    }
    for row in rows.iter().take(limit) {
        let indent = "  ".repeat(row.depth + 1);
        let pct = match by {
            Ranking::SelfTime => row.self_pct,
            Ranking::TotalTime => row.total_pct,
        };
        let total = if row.total_time > 0.0 {
            format!("  total={:.3}s ({:.1}%)", row.total_time, row.total_pct)
        } else {
            String::new()
        };
        println!(
            "{indent}{name}: self={self:.3}s ({pct:.1}%){total}  calls={calls}",
            name = row.name,
            self = row.self_time,
            pct = pct,
            calls = row.calls,
        );
    }
    if rows.len() > limit {
        println!("  ... and {} more", rows.len() - limit);
    }
}
