use ark_bn254::Fr;
use ark_ff::UniformRand;
use clap::Parser;
use light_hash_set::HashSet;
use light_utils::prime::find_next_prime_with_load_factor;
use num_bigint::BigUint;
use rand::thread_rng;
use tabled::{Table, Tabled};

#[derive(Parser)]
pub struct HashSetOptions {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Benchmarks a hash set with given parameters.
    Bench(BenchOptions),
}

#[derive(Parser)]
struct BenchOptions {
    #[clap(long, default_value_t = 4800)]
    expected_capacity: u32,
    #[clap(long, default_value_t = 2800)]
    sequence_threshold: usize,
    #[clap(long, default_values_t = [0.7, 0.5])]
    load_factors: Vec<f64>,
    #[clap(long, default_value = "10")]
    rounds: usize,
}

#[derive(Tabled)]
struct BenchData {
    expected_capacity: u32,
    load_factor: f64,
    capacity_with_load_factor: usize,
    sequence_threshold: usize,
    round: usize,
    successful_insertions: usize,
}

pub(crate) fn hash_set(opts: HashSetOptions) -> anyhow::Result<()> {
    match opts.command {
        Command::Bench(opts) => bench(opts),
    }
}

fn bench(opts: BenchOptions) -> anyhow::Result<()> {
    let BenchOptions {
        expected_capacity,
        sequence_threshold,
        rounds,
        ..
    } = opts;

    let mut result = Vec::new();

    for load_factor in opts.load_factors {
        let capacity_with_load_factor =
            find_next_prime_with_load_factor(opts.expected_capacity, load_factor) as usize;

        let mut hs = HashSet::new(capacity_with_load_factor, opts.sequence_threshold)?;
        let mut rng = thread_rng();

        for round in 0..rounds {
            let mut successful_insertions = capacity_with_load_factor;
            let sequence_number = opts.sequence_threshold * round;

            for element_i in 0..capacity_with_load_factor {
                let value = BigUint::from(Fr::rand(&mut rng));
                match hs.insert(&value, sequence_number) {
                    Ok(index) => hs.mark_with_sequence_number(index, sequence_number)?,
                    Err(_) => {
                        successful_insertions = element_i;
                        break;
                    }
                }
            }

            let bench_data = BenchData {
                expected_capacity,
                load_factor,
                capacity_with_load_factor,
                sequence_threshold,
                round,
                successful_insertions,
            };
            result.push(bench_data);
        }
    }

    let table = Table::new(result);
    println!("{table}");

    Ok(())
}
