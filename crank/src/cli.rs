use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about=None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}
#[derive(Subcommand)]
pub enum Commands {
    NullifyCompressedAccounts {
        #[arg(short, long)]
        nullifier_queue_pubkey: String,
        #[arg(short, long)]
        merkle_tree_pubkey: String,
    },
    SubscribeNullify {
        #[arg(short, long)]
        nullifier_queue_pubkey: String,
        #[arg(short, long)]
        merkle_tree_pubkey: String,
    },
}
