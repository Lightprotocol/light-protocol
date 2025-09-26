use std::{
    collections::HashMap,
    fs::File,
    io::{prelude::*, BufRead, BufReader},
    process::{Command, Stdio},
};

use clap::{ArgAction, Parser};
use tabled::{Table, Tabled};
pub const DESTINATION: &str = "target/";

#[derive(Debug, Parser)]
pub struct Options {
    /// Select the test to run.
    #[clap(long, action = clap::ArgAction::Append)]
    t: Vec<String>,
    /// Select to run compressed token program tests.
    #[clap(long, action = ArgAction::SetTrue)]
    compressed_token: bool,
    /// Select to run compressed pda program tests.
    #[clap(long, action = ArgAction::SetTrue)]
    compressed_pda: bool,
    /// Select to run account compression program tests.
    #[clap(long, action = ArgAction::SetTrue)]
    account_commpression: bool,
    /// Builds all programs with the bench-sbf feature.
    #[clap(long, action = ArgAction::SetTrue)]
    build: bool,
    /// Prints the test logs to the console.
    #[clap(long, action = ArgAction::SetTrue)]
    verbose: bool,
    /// Skips all logs until the start_ix is found.
    #[clap(long, action = clap::ArgAction::Append)]
    start_ix: Option<String>,
}

/// cargo xtask bench --t test_8_transfer  --compressed-token --build --start-ix Transfer --verbose
/// cargo xtask bench --t 1_mint_to  --compressed-token --build
pub fn bench(opts: Options) -> anyhow::Result<()> {
    let (program, program_id) = if opts.compressed_token {
        (
            "light-compressed-token",
            "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m",
        )
    } else if opts.compressed_pda {
        (
            "light-system-program",
            "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7",
        )
    } else if opts.account_commpression {
        (
            "account-compression",
            "CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK",
        )
    } else {
        Err(anyhow::anyhow!("No program selected"))?
    };
    if opts.build {
        println!("Running anchor build");
        Command::new("anchor")
            .args(["build", "--", "--features", "bench-sbf"])
            .stdout(Stdio::piped())
            .output()?;
    }
    for test_name in opts.t {
        println!("Running test: {}", test_name);
        println!("program: {}", program);
        let mut command_output = Command::new("cargo")
            .args([
                "test-sbf",
                "-p",
                program,
                "--features",
                "bench-sbf",
                "--",
                "--test",
                test_name.as_str(),
            ])
            // SVM logs are emitted via sdt err
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = command_output
            .stderr
            .take()
            .expect("Failed to capture stdout");
        let reader = BufReader::new(stdout);
        let output_lines = reader.lines().map(|line| line.unwrap()).collect();

        println!("Creating report for: {}", test_name);
        create_bench_report(
            output_lines,
            test_name,
            program_id,
            opts.verbose,
            &opts.start_ix,
        )?;
    }
    Ok(())
}

pub fn create_bench_report(
    mut output_lines: Vec<String>,
    report_name: String,
    program_id: &str,
    verbose: bool,
    start_ix: &Option<String>,
) -> anyhow::Result<()> {
    // HashMap to store the start and end benchmark details
    let mut benchmarks = HashMap::<String, (u64, u64, u64, u64, u64)>::new();
    let mut expect_sol_log = false;
    let mut start = false;
    let mut end = false;
    let mut found_start = start_ix.is_none();
    let mut current_name = String::new();
    let mut counter = 0;
    for line in output_lines.iter() {
        if verbose {
            println!("{}", line);
        }
        if start_ix.is_some() && !found_start {
            if line.contains(start_ix.as_ref().unwrap()) {
                found_start = true;
            } else {
                continue;
            }
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if expect_sol_log {
            let mem_start_pos_minus_one = parts.iter().position(|&s| s == "remaining").unwrap();

            let mem_start = parts
                .get(mem_start_pos_minus_one - 2)
                .unwrap()
                .parse::<u64>()
                .unwrap();
            expect_sol_log = false;
            if start {
                benchmarks.get_mut(&current_name).unwrap().0 = mem_start;
                start = false;
            }
            if end {
                benchmarks.get_mut(&current_name).unwrap().2 = mem_start;
                end = false;
            }
        }
        if line.contains("_start_bench_cu:") {
            let suffix = "_start_bench_cu:";
            let name = parts
                .iter()
                .find(|&&s| s.ends_with(suffix))
                .map(|s| &s[..s.len() - suffix.len()])
                .unwrap();
            let mem_start_pos_minus_one = parts.iter().position(|&s| s == "used:").unwrap();

            let mem_start = parts
                .get(mem_start_pos_minus_one + 1)
                .unwrap()
                .parse::<u64>()
                .unwrap();
            expect_sol_log = true;
            start = true;
            current_name = name.to_string();
            benchmarks.insert(name.to_string(), (0, mem_start, 0, 0, counter));
            counter += 1;
        } else if line.contains("_end_bench_cu:") {
            let suffix = "_end_bench_cu:";
            let name = parts
                .iter()
                .find(|&&s| s.ends_with(suffix))
                .map(|s| &s[..s.len() - suffix.len()])
                .unwrap();
            expect_sol_log = true;
            end = true;
            current_name = name.to_string();
            let mem_end_pos_minus_one = parts.iter().position(|&s| s == "used:").unwrap();

            let mem_end = parts
                .get(mem_end_pos_minus_one + 1)
                .unwrap()
                .parse::<u64>()
                .unwrap();
            if let Some(value) = benchmarks.get_mut(name) {
                value.3 = mem_end;
            }
        }
    }
    output_lines.reverse();
    let total_cu = match find_total_compute_units(program_id, &output_lines) {
        Some(val) => val,
        None => {
            println!("lines: {:?}", output_lines);
            panic!("Error: Total compute units not found");
        }
    };

    let mut rows = Vec::new();
    rows.push(RowData {
        name: "Total CU".into(),
        cu_percentage: "".into(),
        cu_pre: format_number_with_commas(total_cu),
        cu_post: "".into(),
        cu_used: "".into(),
        memory_used: "".into(),
        memory_start: "".into(),
        memory_end: "".into(),
    });

    rows.push(RowData {
        name: "Name".into(),
        cu_percentage: "CU Percentage".into(),
        cu_pre: "CU Pre".into(),
        cu_post: "CU Post".into(),
        cu_used: "CU Used".into(),
        memory_used: "Memory Used".into(),
        memory_start: "Memory Start".into(),
        memory_end: "Memory End".into(),
    });

    #[allow(clippy::type_complexity)]
    let mut sorted_benchmarks: Vec<(String, (u64, u64, u64, u64, u64))> = benchmarks
        .iter()
        .map(|(name, values)| (name.clone(), *values))
        .collect();

    #[allow(clippy::clone_on_copy)]
    sorted_benchmarks.sort_by_key(|(_, (_, _, _, _, position))| position.clone());
    for (name, (cu_pre, mem_start, cu_post, mem_end, _)) in benchmarks {
        let cu_used = cu_pre - cu_post;
        let memory_used = match mem_end.checked_sub(mem_start) {
            Some(val) => val,
            None => {
                panic!("Error: Memory end is less than memory start for {}", name);
            }
        };
        let cu_percentage = (cu_used as f64 / total_cu as f64) * 100.0;
        rows.push(RowData {
            name,
            cu_percentage: format!("{:.2}", cu_percentage),
            cu_pre: format_number_with_commas(cu_pre),
            cu_post: format_number_with_commas(cu_post),
            cu_used: format_number_with_commas(cu_used),
            memory_used: format_number_with_commas(memory_used),
            memory_start: format_number_with_commas(mem_start),
            memory_end: format_number_with_commas(mem_end),
        });
    }
    let path = DESTINATION.to_string() + report_name.as_str() + ".txt";
    let mut file = File::create(path.clone())?;
    let table = Table::new(rows);
    write!(file, "{}", table)?;
    println!("Writing report to: {}", path);
    Ok(())
}
#[derive(Tabled)]
struct RowData {
    name: String,
    cu_percentage: String,
    cu_pre: String,
    cu_post: String,
    cu_used: String,
    memory_used: String,
    memory_start: String,
    memory_end: String,
}
fn format_number_with_commas(num: u64) -> String {
    let num_str = num.to_string();
    let mut result = String::new();
    let digits = num_str.len();

    num_str.chars().enumerate().for_each(|(i, c)| {
        if (digits - i).is_multiple_of(3) && i != 0 {
            result.push(',');
        }
        result.push(c);
    });

    result
}

fn find_total_compute_units(program_id: &str, logs: &Vec<String>) -> Option<u64> {
    // Iterate through each log entry
    for log in logs {
        if log.contains(program_id) && log.contains("consumed") {
            // Split the log line into parts
            let parts: Vec<&str> = log.split_whitespace().collect();
            // Find the position of "consumed" and get the next element which should be the number
            if let Some(index) = parts.iter().position(|&x| x == "consumed") {
                if let Some(consumed_units) = parts.get(index + 1) {
                    // Attempt to parse the number and return it
                    return consumed_units.parse::<u64>().ok();
                }
            }
        }
    }
    // Return None if no matching log entry was found or if any part of the process failed
    None
}
