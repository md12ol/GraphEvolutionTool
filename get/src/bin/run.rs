//! `get-run` — drive an evolution run straight from a `config.toml`, no
//! Python interpreter and no `get.so` build required.
//!
//! Mirrors the steps a Python caller takes (`GraphEvolver::new` → `run` →
//! `save_logs`/`save_results`) through [`get::run_from_toml`], so a run's
//! output files are identical whichever front end produced them.

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        eprintln!("usage: {} <config.toml> [seed]", args[0]);
        return ExitCode::FAILURE;
    }

    let config_path = &args[1];
    let seed: u64 = match args.get(2) {
        Some(raw) => match raw.parse() {
            Ok(seed) => seed,
            Err(_) => {
                eprintln!("seed must be a non-negative integer, got \"{raw}\"");
                return ExitCode::FAILURE;
            }
        },
        None => rand::random(),
    };

    println!("config = {config_path}, seed = {seed}");

    let summary = match get::run_from_toml(config_path, seed) {
        Ok(summary) => summary,
        Err(err) => {
            eprintln!("run failed: {err}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "\nbest_fitness = {}\nedges = {}\ngenome = {}",
        summary.best_fitness,
        summary.best_edges.len(),
        summary.best_genome_repr,
    );

    println!("\nconvergence log ({} rows):", summary.history.len());
    println!(
        "{:>10} {:>14} {:>14} {:>10} {:>10}",
        "iteration", "best_fitness", "mean_fitness", "std_dev", "ci_95"
    );
    for row in &summary.history {
        println!(
            "{:>10} {:>14.6} {:>14.6} {:>10.6} {:>10.6}",
            row.iteration, row.best_fitness, row.mean_fitness, row.std_dev, row.ci_95,
        );
    }

    match summary.save_logs("run_log.csv") {
        Ok(()) => println!("\nwrote run_log.csv"),
        Err(err) => eprintln!("\nwarning: could not write run_log.csv: {err}"),
    }
    match summary.save_results("best_individual.txt") {
        Ok(()) => println!("wrote best_individual.txt (+ .toml)"),
        Err(err) => eprintln!("warning: could not write best_individual.txt: {err}"),
    }

    ExitCode::SUCCESS
}
