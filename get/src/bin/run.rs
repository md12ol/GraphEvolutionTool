//! `get-run` — drive an evolution run from a `config.toml`, with no Python
//! interpreter and no built extension module.
//!
//! Files land in the working directory under fixed names, or under `--out DIR`
//! in `DIR/<timestamp>-<seed>/`. There, replicates each get a `run_<index>/`
//! sub-directory; a single run's files sit in the timestamped directory itself.

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
  --runs N    number of replicates from that master seed (default 1). More
              than one requires --out, which is what keeps them apart.
  --out DIR   write into DIR/<timestamp>-<seed>/ instead of the working
              directory, so nothing is overwritten between invocations.";

/// Parse the command line, or say what was wrong with it.
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

    if n_runs > 1 && out_dir.is_none() {
        return Err(format!(
            "--runs {n_runs} needs --out DIR; without it every replicate would \
             overwrite the last in the working directory"
        ));
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

    // Before parsing, so `--help` works without a config file.
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

    // The only record of a seed that was omitted and drawn at random.
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
            // Zero-based: `run_index` is half of the pair that reproduces a
            // replicate, with the master seed.
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

    // Success even if a write above failed: the run itself finished, and its
    // results are on stdout whether or not they reached a file.
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    /// `argv[0]` is the program name, which `parse_args` skips.
    fn args(rest: &[&str]) -> Vec<String> {
        let mut argv = vec!["get-run".to_string()];
        for arg in rest {
            argv.push((*arg).to_string());
        }
        argv
    }

    #[test]
    fn several_replicates_without_out_is_rejected() {
        // `unwrap_err` would need `Args: Debug`; matching keeps the test's
        // needs out of the type the binary actually uses.
        match parse_args(&args(&["c.toml", "7", "--runs", "3"])) {
            Ok(_) => panic!("--runs 3 without --out should be rejected"),
            Err(err) => assert!(err.contains("--out"), "error should name --out: {err}"),
        }
    }

    #[test]
    fn several_replicates_with_out_is_accepted() {
        let parsed = parse_args(&args(&["c.toml", "7", "--runs", "3", "--out", "d"])).unwrap();
        assert_eq!(parsed.n_runs, 3);
        assert_eq!(parsed.out_dir.as_deref(), Some("d"));
    }

    /// The flat working-directory layout is still the default, and CI's route
    /// check runs exactly this way.
    #[test]
    fn a_single_run_needs_no_out() {
        let parsed = parse_args(&args(&["c.toml", "7"])).unwrap();
        assert_eq!(parsed.n_runs, 1);
        assert!(parsed.out_dir.is_none());
    }

    #[test]
    fn zero_replicates_is_rejected() {
        assert!(parse_args(&args(&["c.toml", "--runs", "0"])).is_err());
    }
}
