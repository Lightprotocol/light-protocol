mod write_lookup_table;

use write_lookup_table::build_lookup_table;
use clap::Parser;

#[derive(Parser)]
pub struct XtaskOptions {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    WriteLookupTable(write_lookup_table::Args),
}

fn main() -> Result<(), anyhow::Error> {
        let args = XtaskOptions::parse();
        match args.command {
            Command::WriteLookupTable(args) => {
                // Call your function with the parsed usize value
                build_lookup_table(args);
            }
        }

        Ok(())   
}
