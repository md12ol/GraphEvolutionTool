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
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

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

/// `YYYYmmdd-HHMMSS` in UTC, for naming a run's output directory.
///
/// UTC rather than local time, so directories from two machines sort into the
/// order the runs actually happened. Converted here rather than through a date
/// crate: one directory name does not justify a dependency, and the arithmetic
/// below is the standard civil-from-days algorithm, exact for every date this
/// program will ever see.
fn utc_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);

    let days = (secs / 86_400) as i64;
    let time_of_day = secs % 86_400;

    // Shift the epoch to 0000-03-01, which puts the leap day at the end of the
    // year and makes the month arithmetic below a single linear formula.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;

    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        time_of_day / 3_600,
        (time_of_day % 3_600) / 60,
        time_of_day % 60,
    )
}

/// Where one replicate's files belong, creating the directory if needed.
///
/// `None` for `out_dir` keeps the historical behaviour — fixed names in the
/// working directory — because the CI route check and every existing note about
/// this binary expect them there.
///
/// `stamp` is passed in rather than read here: every replicate of one
/// invocation belongs in the same timestamped directory, and reading the clock
/// per replicate would scatter them the moment a run crossed a second boundary.
fn output_dir(
    out_dir: Option<&str>,
    stamp: &str,
    seed: u64,
    run_index: usize,
    n_runs: usize,
) -> PathBuf {
    let Some(root) = out_dir else {
        return PathBuf::from(".");
    };

    let mut path = Path::new(root).join(format!("{stamp}-{seed}"));
    if n_runs > 1 {
        path = path.join(format!("run_{run_index}"));
    }
    path
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().collect();
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

    let stamp = utc_stamp();

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

        let directory = output_dir(
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
