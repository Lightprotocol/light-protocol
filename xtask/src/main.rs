use clap::{Parser, ValueEnum};

mod bench;
mod create_vkeyrs_from_gnark_key;
mod fee;
mod type_sizes;
mod zero_bytes;
mod zero_indexed_leaf;

#[derive(Debug, Clone, ValueEnum)]
enum Hash {
    Keccak,
    Poseidon,
    Sha256,
}

#[derive(Parser)]
pub struct XtaskOptions {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    GenerateZeroBytes(zero_bytes::Options),
    /// Generates a leaf of an indexed Merkle tree, for the given hash, which
    /// represents a value 0.
    GenerateZeroIndexedLeaf(zero_indexed_leaf::Options),
    /// Shows the sizes of types used as Light Protocol accounts (or their
    /// fields).
    TypeSizes,
    /// Generates the verification keys for the given gnark key.
    GenerateVkeyRs(create_vkeyrs_from_gnark_key::Options),
    /// Generates cu and heap memory usage report from a log.txt file
    Bench(bench::Options),
    /// Prints fees for different accounts.
    Fee,
}

fn main() -> Result<(), anyhow::Error> {
    let opts = XtaskOptions::parse();

    match opts.command {
        Command::TypeSizes => type_sizes::type_sizes(),
        Command::GenerateZeroBytes(opts) => zero_bytes::generate_zero_bytes(opts),
        Command::GenerateZeroIndexedLeaf(opts) => {
            zero_indexed_leaf::generate_zero_indexed_leaf(opts)
        }
        Command::GenerateVkeyRs(opts) => {
            create_vkeyrs_from_gnark_key::create_vkeyrs_from_gnark_key(opts)
        }
        Command::Bench(opts) => bench::bench(opts),
        Command::Fee => fee::fees(),
    }
}
