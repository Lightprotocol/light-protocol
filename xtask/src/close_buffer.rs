use clap::Parser;
use solana_sdk::{bpf_loader_upgradeable, bs58, message::Message, pubkey::Pubkey};
use std::str::FromStr;

#[derive(Debug, Parser)]
pub struct Options {
    /// The buffer account pubkey to close
    #[clap(long)]
    buffer: String,
    /// The multisig authority pubkey
    #[clap(long, default_value = "7PeqkcCXeqgsp5Mi15gjJh8qvSLk7n3dgNuyfPhJJgqY")]
    authority: String,
    /// The recipient pubkey for reclaimed lamports
    #[clap(long)]
    recipient: String,
}

/// Creates a serialized BPF Loader Close instruction for use with Squads TX builder.
///
/// Steps:
/// 1. Build the close instruction for the BPF Upgradeable Loader
/// 2. Serialize the message to bs58
/// 3. Print bs58 for use in Squads
pub fn close_buffer(options: Options) -> anyhow::Result<()> {
    let buffer = Pubkey::from_str(&options.buffer)
        .map_err(|e| anyhow::anyhow!("Invalid buffer pubkey: {e}"))?;
    let authority = Pubkey::from_str(&options.authority)
        .map_err(|e| anyhow::anyhow!("Invalid authority pubkey: {e}"))?;
    let recipient = Pubkey::from_str(&options.recipient)
        .map_err(|e| anyhow::anyhow!("Invalid recipient pubkey: {e}"))?;

    let instruction =
        bpf_loader_upgradeable::close_any(&buffer, &recipient, Some(&authority), None);

    println!("instruction: {:?}", instruction);
    println!(
        "Serialized instruction data: {}",
        bs58::encode(instruction.data.clone()).into_string()
    );
    let message = Message::new(&[instruction], Some(&authority));
    let serialized_message = bs58::encode(message.serialize()).into_string();
    println!(
        "\n ----------- Use the serialized message in the squads tx builder. --------------- \n"
    );
    println!("serialized message: {}", serialized_message);

    Ok(())
}
