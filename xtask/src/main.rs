use clap::Parser;

mod zero_bytes;

#[derive(Parser)]
pub struct XtaskOptions {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    GenerateZeroBytes(zero_bytes::Options),
}

fn main() -> Result<(), anyhow::Error> {
    let opts = XtaskOptions::parse();

    match opts.command {
        Command::GenerateZeroBytes(opts) => zero_bytes::generate_zero_bytes(opts)?,
    }

    Ok(())
}
