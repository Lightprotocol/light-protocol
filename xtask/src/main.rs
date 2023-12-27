use clap::{Parser, ValueEnum};

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
    /// Generates a leaf of an indexed Merkle tree, for the given hash, which
    /// represents a value 0.
    GenerateZeroIndexedLeaf(zero_indexed_leaf::Options),
    GenerateZeroBytes(zero_bytes::Options),
}

fn main() -> Result<(), anyhow::Error> {
    let opts = XtaskOptions::parse();

    match opts.command {
        Command::GenerateZeroIndexedLeaf(opts) => {
            zero_indexed_leaf::generate_zero_indexed_leaf(opts)?
        }
        Command::GenerateZeroBytes(opts) => zero_bytes::generate_zero_bytes(opts)?,
    }

    Ok(())
}
