use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about=None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}
#[derive(Subcommand)]
pub enum Commands {
    StateQueueInfo,
    AddressQueueInfo,
    Airdrop,
    NullifyState,
    NullifyAddresses,
    Nullify,
    Subscribe,
    TreeSync,
}
