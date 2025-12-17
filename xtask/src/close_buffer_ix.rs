use clap::Parser;
use solana_loader_v3_interface::instruction::close;
use solana_sdk::{bs58, message::Message, pubkey::Pubkey};
use std::str::FromStr;

#[derive(Debug, Parser)]
pub struct Options {
    /// Buffer account pubkey to close
    #[clap(long)]
    buffer: String,
    /// Recipient to receive the lamports (defaults to authority/multisig)
    #[clap(long)]
    recipient: Option<String>,
}

pub fn close_buffer_ix(options: Options) -> anyhow::Result<()> {
    let authority = solana_sdk::pubkey!("7PeqkcCXeqgsp5Mi15gjJh8qvSLk7n3dgNuyfPhJJgqY");
    let buffer = Pubkey::from_str(&options.buffer)?;
    let recipient = options
        .recipient
        .map(|r| Pubkey::from_str(&r))
        .transpose()?
        .unwrap_or(authority);

    let instruction = close(&buffer, &recipient, &authority);

    println!("Close buffer instruction:");
    println!("  Buffer: {}", buffer);
    println!("  Recipient: {}", recipient);
    println!("  Authority: {}", authority);

    let message = Message::new(&[instruction], Some(&authority));
    let serialized_message = bs58::encode(message.serialize()).into_string();

    println!("\n----------- Use in Squads tx builder -----------\n");
    println!("Serialized message: {}", serialized_message);

    Ok(())
}
