use std::path::PathBuf;

use anchor_lang::{InstructionData, ToAccountMetas};
use clap::Parser;
use dirs::home_dir;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use light_compressible::{
    config::COMPRESSIBLE_CONFIG_SEED,
    registry_instructions::CreateCompressibleConfig as CreateCompressibleConfigParams,
    rent::RentConfig,
};
use light_registry::{
    compressible::create_config_counter::COMPRESSIBLE_CONFIG_COUNTER_SEED,
    utils::get_protocol_config_pda_address,
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
};

const REGISTRY_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");

#[derive(Debug, Parser)]
pub struct Options {
    /// Path to payer keypair file (defaults to ~/.config/solana/id.json)
    #[clap(long)]
    payer: Option<PathBuf>,

    /// Network: devnet, mainnet, local, or custom RPC URL (default: devnet)
    #[clap(long, default_value = "devnet")]
    network: String,

    /// Skip config counter creation (if it already exists)
    #[clap(long, default_value = "false")]
    skip_counter: bool,
}

fn get_rpc_url(network: &str) -> String {
    match network {
        "local" => String::from("http://127.0.0.1:8899"),
        "devnet" => String::from("https://api.devnet.solana.com"),
        "mainnet" => String::from("https://api.mainnet-beta.solana.com"),
        other => other.to_string(),
    }
}

fn get_config_counter_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[COMPRESSIBLE_CONFIG_COUNTER_SEED], &REGISTRY_PROGRAM_ID)
}

fn get_compressible_config_pda(version: u16) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[COMPRESSIBLE_CONFIG_SEED, &version.to_le_bytes()],
        &REGISTRY_PROGRAM_ID,
    )
}

fn create_config_counter_ix(fee_payer: Pubkey, authority: Pubkey) -> Instruction {
    let (protocol_config_pda, _) = get_protocol_config_pda_address();
    let (config_counter_pda, _) = get_config_counter_pda();

    let accounts = light_registry::accounts::CreateConfigCounter {
        fee_payer,
        authority,
        protocol_config_pda,
        config_counter: config_counter_pda,
        system_program: anchor_lang::system_program::ID,
    };

    let instruction_data = light_registry::instruction::CreateConfigCounter {};

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

fn create_compressible_config_ix(
    fee_payer: Pubkey,
    authority: Pubkey,
    update_authority: Pubkey,
    withdrawal_authority: Pubkey,
    version: u16,
) -> Instruction {
    let (protocol_config_pda, _) = get_protocol_config_pda_address();
    let (config_counter_pda, _) = get_config_counter_pda();
    let (compressible_config_pda, _) = get_compressible_config_pda(version);

    let accounts = light_registry::accounts::CreateCompressibleConfig {
        fee_payer,
        authority,
        protocol_config_pda,
        config_counter: config_counter_pda,
        compressible_config: compressible_config_pda,
        system_program: anchor_lang::system_program::ID,
    };

    let instruction_data = light_registry::instruction::CreateCompressibleConfig {
        params: CreateCompressibleConfigParams {
            rent_config: RentConfig::default(),
            update_authority,
            withdrawal_authority,
            active: true,
        },
    };

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub async fn create_compressible_config(options: Options) -> anyhow::Result<()> {
    let rpc_url = get_rpc_url(&options.network);
    println!("Connecting to: {}", rpc_url);

    let mut rpc = LightClient::new(LightClientConfig {
        url: rpc_url,
        photon_url: None,
        commitment_config: None,
        fetch_active_tree: false,
        api_key: None,
    })
    .await?;

    // Load payer keypair
    let payer = if let Some(payer_path) = options.payer.as_ref() {
        read_keypair_file(payer_path).unwrap_or_else(|_| panic!("Failed to read {:?}", payer_path))
    } else {
        let keypair_path: PathBuf = home_dir()
            .expect("Could not find home directory")
            .join(".config/solana/id.json");
        read_keypair_file(&keypair_path)
            .unwrap_or_else(|_| panic!("Keypair not found in default path {:?}", keypair_path))
    };
    println!("Payer: {:?}", payer.pubkey());

    let balance = rpc.get_balance(&payer.pubkey()).await?;
    println!(
        "Payer balance: {} lamports ({} SOL)",
        balance,
        balance as f64 / 1e9
    );

    // Get protocol config PDA to fetch the authority
    let (protocol_config_pda, _) = get_protocol_config_pda_address();
    println!("Protocol config PDA: {:?}", protocol_config_pda);

    let protocol_config_account = rpc
        .get_account(protocol_config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Protocol config account not found"))?;

    // Extract authority from account data:
    // - 8 bytes discriminator
    // - 32 bytes authority pubkey
    if protocol_config_account.data.len() < 40 {
        return Err(anyhow::anyhow!("Protocol config account data too short"));
    }
    let protocol_authority = Pubkey::try_from(&protocol_config_account.data[8..40])
        .map_err(|_| anyhow::anyhow!("Failed to parse protocol authority pubkey"))?;
    println!("Protocol authority: {:?}", protocol_authority);

    // Check that payer is the protocol authority
    if payer.pubkey() != protocol_authority {
        return Err(anyhow::anyhow!(
            "Payer ({}) is not the protocol authority ({}). \
            The protocol authority must sign these transactions.",
            payer.pubkey(),
            protocol_authority
        ));
    }

    let (config_counter_pda, _) = get_config_counter_pda();
    println!("Config counter PDA: {:?}", config_counter_pda);

    // Step 1: Create config counter if it doesn't exist
    if !options.skip_counter {
        let counter_account = rpc.get_account(config_counter_pda).await?;
        if counter_account.is_none() {
            println!("\n=== Creating Config Counter ===");
            let create_counter_ix = create_config_counter_ix(payer.pubkey(), payer.pubkey());

            let signature = rpc
                .create_and_send_transaction(&[create_counter_ix], &payer.pubkey(), &[&payer])
                .await?;
            println!("Config counter created! Signature: {:?}", signature);
        } else {
            println!("Config counter already exists, skipping creation.");
        }
    }

    // Step 2: Create compressible config with version 1
    let version: u16 = 1;
    let (compressible_config_pda, _) = get_compressible_config_pda(version);
    println!("\n=== Creating Compressible Config ===");
    println!("Compressible config PDA: {:?}", compressible_config_pda);

    // Check if config already exists
    let config_account = rpc.get_account(compressible_config_pda).await?;
    if config_account.is_some() {
        println!(
            "Compressible config already exists at {:?}",
            compressible_config_pda
        );
        return Ok(());
    }

    let create_config_ix = create_compressible_config_ix(
        payer.pubkey(),
        payer.pubkey(),
        protocol_authority,
        protocol_authority,
        version,
    );

    let signature = rpc
        .create_and_send_transaction(&[create_config_ix], &payer.pubkey(), &[&payer])
        .await?;
    println!("Compressible config created! Signature: {:?}", signature);

    println!("\n=== Summary ===");
    println!("Network: {}", options.network);
    println!("Config counter PDA: {:?}", config_counter_pda);
    println!("Compressible config PDA: {:?}", compressible_config_pda);
    println!("Update authority: {:?}", protocol_authority);
    println!("Withdrawal authority: {:?}", protocol_authority);
    println!("Rent config: {:?}", RentConfig::default());

    Ok(())
}
