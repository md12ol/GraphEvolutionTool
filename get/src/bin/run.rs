//! `get-run` — drive an evolution run straight from a `config.toml`, no
//! Python interpreter and no `get.so` build required.
//!
//! Mirrors the steps a Python caller takes (`GraphEvolver::new` → `run` →
//! `save_logs`/`save_results`) through [`get::run_many_from_toml`], so a run's
//! output files are identical whichever front end produced them.
//!
//! # Where the files go
//!
//! With no `--out`, into the working directory under the fixed names
//! `run_log.csv` and `best_individual.txt` (+ `.toml`) — which means a second
//! invocation overwrites the first, and four runs need four directories.
//!
//! With `--out <dir>`, into `<dir>/<timestamp>-<seed>/`, one directory per
//! invocation, so nothing is ever overwritten and the directory name says which
//! run it was. Several replicates get a `run_<index>/` sub-directory each; a
//! single run's files sit directly in the timestamped directory, there being
//! nothing to tell apart.

use std::env;
use std::process::ExitCode;

/// What the command line asked for.
struct Args {
    config_path: String,
    seed: u64,
    n_runs: usize,
    out_dir: Option<String>,
}

const USAGE: &str = "usage: get-run <config.toml> [seed] [--runs N] [--out DIR]

  seed        master seed; random if omitted. Replicate `i` is reproduced by
              re-running with the same master seed and reading run_<i>.
  --runs N    number of replicates from that master seed (default 1).
  --out DIR   write into DIR/<timestamp>-<seed>/ instead of the working
              directory, so nothing is overwritten between invocations.";

/// Parse the command line, or say what was wrong with it.
///
/// Hand-rolled rather than pulled from a crate: two flags and two positionals
/// do not earn a dependency, and this binary is the one place in the tree that
/// reads `argv` at all.
fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut positional: Vec<&String> = Vec::new();
    let mut n_runs = 1usize;
    let mut out_dir: Option<String> = None;

    let mut i = 1;
    while i < argv.len() {
        let arg = &argv[i];
        if arg == "--runs" || arg == "--out" {
            let value = match argv.get(i + 1) {
                Some(value) => value,
                None => return Err(format!("{arg} needs a value")),
            };
            if arg == "--runs" {
                n_runs = match value.parse() {
                    Ok(0) => return Err("--runs must be at least 1".to_string()),
                    Ok(parsed) => parsed,
                    Err(_) => {
                        return Err(format!("--runs must be a positive integer, got {value:?}"));
                    }
                };
            } else {
                out_dir = Some(value.clone());
            }
            i += 2;
        } else if arg.starts_with("--") {
            return Err(format!("unknown option {arg:?}"));
        } else {
            positional.push(arg);
            i += 1;
        }
    }

    if positional.is_empty() || positional.len() > 2 {
        return Err("expected a config file and an optional seed".to_string());
    }

    let seed = match positional.get(1) {
        Some(raw) => match raw.parse() {
            Ok(seed) => seed,
            Err(_) => return Err(format!("seed must be a non-negative integer, got {raw:?}")),
        },
        None => rand::random(),
    };

    Ok(Args {
        config_path: positional[0].clone(),
        seed,
        n_runs,
        out_dir,
    })
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().collect();

    // Before parsing, so `--help` works without a config file. It is the first
    // thing a new user runs, and printing usage to stderr with a failing exit
    // code is the wrong answer to a question that was asked correctly.
    if argv[1..].iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    let args = match parse_args(&argv) {
        Ok(args) => args,
        Err(problem) => {
            eprintln!("{problem}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "config = {}, seed = {}, runs = {}",
        args.config_path, args.seed, args.n_runs
    );

    let stamp = get::utc_stamp();

    let summaries = match get::run_many_from_toml(&args.config_path, args.seed, args.n_runs) {
        Ok(summaries) => summaries,
        Err(err) => {
            eprintln!("run failed: {err}");
            return ExitCode::FAILURE;
        }
    };

    for (run_index, summary) in summaries.iter().enumerate() {
        if args.n_runs > 1 {
            // Zero-based, because `run_index` is half of the pair that
            // reproduces a replicate — printing it 1-based would invite someone
            // to ask for the wrong one.
            println!("\n=== run_index {run_index}, of {} ===", args.n_runs);
        }

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

        let directory = get::run_output_dir(
            args.out_dir.as_deref(),
            &stamp,
            args.seed,
            run_index,
            args.n_runs,
        );
        if let Err(err) = std::fs::create_dir_all(&directory) {
            eprintln!("\nwarning: could not create {}: {err}", directory.display());
            continue;
        }

        let log_path = directory.join("run_log.csv");
        match summary.save_logs(&log_path.to_string_lossy()) {
            Ok(()) => println!("\nwrote {}", log_path.display()),
            Err(err) => eprintln!("\nwarning: could not write {}: {err}", log_path.display()),
        }

        let best_path = directory.join("best_individual.txt");
        match summary.save_results(&best_path.to_string_lossy()) {
            Ok(()) => println!("wrote {} (+ .toml)", best_path.display()),
            Err(err) => eprintln!("warning: could not write {}: {err}", best_path.display()),
        }
    }

    ExitCode::SUCCESS
}
