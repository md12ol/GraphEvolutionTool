//! `get-run` — drive an evolution run from a `config.toml`, with no Python
//! interpreter and no built extension module. `USAGE` below is the whole
//! interface.

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

Writes run_log.csv, best_individual.txt and config.toml into a
<timestamp>-<seed>/ folder, made in the working directory or under DIR with
--out. Each replicate of a multi-run invocation gets its own run_<n>/ inside it.

  seed        master seed; random if omitted. Replicate `i` is reproduced by
              re-running with the same master seed and reading run_<i>.
  --runs N    number of replicates from that master seed (default 1).
  --out DIR   make the timestamped folder under DIR instead of here. Either way
              nothing is overwritten between invocations.";

/// What `parse_args` found on the command line.
enum Parsed {
    Run(Args),
    HelpWanted,
    Error(String),
}

/// Parse the command line, or say what was wrong with it.
fn parse_args(argv: &[String]) -> Parsed {
    // First, so `--help` works without a config file and wins over any other
    // problem on the line, including a flag that would take it as its value.
    for arg in &argv[1..] {
        if arg == "--help" || arg == "-h" {
            return Parsed::HelpWanted;
        }
    }

    let mut positional: Vec<&String> = Vec::new();
    let mut n_runs = 1usize;
    let mut out_dir: Option<String> = None;

    let mut i = 1;
    while i < argv.len() {
        let arg = &argv[i];
        if arg == "--runs" || arg == "--out" {
            let value = match argv.get(i + 1) {
                Some(value) => value,
                None => return Parsed::Error(format!("{arg} needs a value")),
            };
            if arg == "--runs" {
                n_runs = match value.parse() {
                    Ok(0) => return Parsed::Error("--runs must be at least 1".to_string()),
                    Ok(parsed) => parsed,
                    Err(_) => {
                        return Parsed::Error(format!(
                            "--runs must be a positive integer, got {value:?}"
                        ));
                    }
                };
            } else {
                out_dir = Some(value.clone());
            }
            i += 2;
        } else if arg.starts_with("--") {
            return Parsed::Error(format!("unknown option {arg:?}"));
        } else {
            positional.push(arg);
            i += 1;
        }
    }

    if positional.is_empty() || positional.len() > 2 {
        return Parsed::Error("expected a config file and an optional seed".to_string());
    }

    let seed = match positional.get(1) {
        Some(raw) => match raw.parse() {
            Ok(seed) => seed,
            Err(_) => {
                return Parsed::Error(format!("seed must be a non-negative integer, got {raw:?}"));
            }
        },
        None => rand::random(),
    };

    Parsed::Run(Args {
        config_path: positional[0].clone(),
        seed,
        n_runs,
        out_dir,
    })
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().collect();

    let args = match parse_args(&argv) {
        Parsed::Run(args) => args,
        Parsed::HelpWanted => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Parsed::Error(problem) => {
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

    // Once per invocation, not once per replicate: every summary carries the
    // same document, so this is the folder they all share. Written before the
    // loop so it lands even if a later replicate's own writes fail.
    if let Some(first) = summaries.first() {
        let experiment = get::experiment_output_dir(args.out_dir.as_deref(), &stamp, args.seed);
        match std::fs::create_dir_all(&experiment).and_then(|()| first.save_config(&experiment)) {
            Ok(()) => println!("\nwrote {}", experiment.join("config.toml").display()),
            Err(err) => eprintln!(
                "\nwarning: could not write {}: {err}",
                experiment.join("config.toml").display()
            ),
        }
    }

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
            Ok(()) => println!("wrote {}", best_path.display()),
            Err(err) => eprintln!("warning: could not write {}: {err}", best_path.display()),
        }
    }

    // Success even if a write above failed: the run itself finished, and its
    // results are on stdout whether or not they reached a file.
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{Args, Parsed, parse_args};

    /// `argv[0]` is the program name, which `parse_args` skips.
    fn args(rest: &[&str]) -> Vec<String> {
        let mut argv = vec!["get-run".to_string()];
        for arg in rest {
            argv.push((*arg).to_string());
        }
        argv
    }

    /// Parse a command line that is expected to start a run.
    fn run(rest: &[&str]) -> Args {
        match parse_args(&args(rest)) {
            Parsed::Run(parsed) => parsed,
            Parsed::HelpWanted => panic!("{rest:?} asked for help, expected a run"),
            Parsed::Error(problem) => panic!("{rest:?} was rejected: {problem}"),
        }
    }

    #[test]
    fn several_replicates_need_no_output_directory() {
        // The timestamped folder is made with or without `--out`, and each
        // replicate writes its own `run_<n>/` inside it, so none can collide.
        let parsed = run(&["c.toml", "7", "--runs", "3"]);
        assert_eq!(parsed.n_runs, 3);
        assert_eq!(parsed.out_dir, None);
    }

    #[test]
    fn several_replicates_with_out_is_accepted() {
        let parsed = run(&["c.toml", "7", "--runs", "3", "--out", "d"]);
        assert_eq!(parsed.n_runs, 3);
        assert_eq!(parsed.out_dir.as_deref(), Some("d"));
    }

    /// The flat working-directory layout is still the default, and CI's route
    /// check runs exactly this way.
    #[test]
    fn a_single_run_needs_no_out() {
        let parsed = run(&["c.toml", "7"]);
        assert_eq!(parsed.n_runs, 1);
        assert!(parsed.out_dir.is_none());
    }

    #[test]
    fn zero_replicates_is_rejected() {
        let parsed = parse_args(&args(&["c.toml", "--runs", "0"]));
        assert!(matches!(parsed, Parsed::Error(_)));
    }

    #[test]
    fn help_needs_no_config_file() {
        assert!(matches!(parse_args(&args(&["--help"])), Parsed::HelpWanted));
        assert!(matches!(parse_args(&args(&["-h"])), Parsed::HelpWanted));
    }

    #[test]
    fn help_wins_over_a_flag_that_would_take_it_as_a_value() {
        let after_runs = parse_args(&args(&["c.toml", "--runs", "--help"]));
        assert!(matches!(after_runs, Parsed::HelpWanted));
        let after_out = parse_args(&args(&["c.toml", "--out", "-h"]));
        assert!(matches!(after_out, Parsed::HelpWanted));
    }
}
