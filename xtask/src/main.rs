use clap::{Parser, ValueEnum};

mod accounts;
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
    Accounts,
    GenerateZeroBytes(zero_bytes::Options),
    /// Generates a leaf of an indexed Merkle tree, for the given hash, which
    /// represents a value 0.
    GenerateZeroIndexedLeaf(zero_indexed_leaf::Options),
}

fn main() -> Result<(), anyhow::Error> {
    let opts = XtaskOptions::parse();

    match opts.command {
        Command::Accounts => accounts::accounts(),
        Command::GenerateZeroBytes(opts) => zero_bytes::generate_zero_bytes(opts),
        Command::GenerateZeroIndexedLeaf(opts) => {
            zero_indexed_leaf::generate_zero_indexed_leaf(opts)
        }
    }
}
