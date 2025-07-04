use account_compression::processor::initialize_address_merkle_tree::AnchorDeserialize;
use clap::Parser;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use light_registry::{
    protocol_config::state::ProtocolConfigPda, sdk::create_update_protocol_config_instruction,
    utils::get_protocol_config_pda_address,
};
use solana_sdk::{bs58, message::Message, pubkey};

/// Updateable Parameters:
/// 1. slot_length
/// 2. cpi_context_size
/// 3. min_weight
#[derive(Debug, Parser)]
pub struct Options {
    #[clap(long)]
    slot_length: Option<u64>,
    #[clap(long)]
    cpi_context_size: Option<u64>,
    #[clap(long)]
    min_weight: Option<u64>,
}

/// Steps:
/// 1. fetch protocol config account
///    1.1. print protocol config account
/// 2. create updated protocol config based on inputs
///    2.1. print updated protocol config
/// 3. create instruction
///    - signer is the multisig
/// 4. serialize instruction to bs58
/// 5. print bs58
pub async fn create_update_protocol_config_ix(options: Options) -> anyhow::Result<()> {
    let rpc_url = String::from("https://api.mainnet-beta.solana.com");
    let rpc = LightClient::new(LightClientConfig {
        url: rpc_url,
        photon_url: None,
        commitment_config: None,
        fetch_active_tree: false,
        api_key: None,
    })
    .await
    .unwrap();
    let (protocol_config_pda, _) = get_protocol_config_pda_address();
    let account = rpc
        .get_account(protocol_config_pda)
        .await?
        .expect("Protocol Config Account not found");
    let mut deserialized_account = ProtocolConfigPda::deserialize(&mut &account.data[8..]).unwrap();
    println!("current protocol config: {:?}", deserialized_account);
    if let Some(slot_length) = options.slot_length {
        deserialized_account.config.slot_length = slot_length;
    }
    if let Some(cpi_context_size) = options.cpi_context_size {
        deserialized_account.config.cpi_context_size = cpi_context_size;
    }
    if let Some(min_weight) = options.min_weight {
        deserialized_account.config.min_weight = min_weight;
    }
    println!("updated protocol config: {:?}", deserialized_account.config);
    let authority = pubkey!("7PeqkcCXeqgsp5Mi15gjJh8qvSLk7n3dgNuyfPhJJgqY");
    let instruction = create_update_protocol_config_instruction(
        authority,
        None,
        Some(deserialized_account.config),
    );
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
