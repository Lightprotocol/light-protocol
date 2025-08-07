use std::{path::PathBuf, str::FromStr};

use anchor_lang::{InstructionData, ToAccountMetas};
use clap::Parser;
use dirs::home_dir;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use solana_sdk::{
    instruction::Instruction,
    signature::{read_keypair_file, Signer},
};

#[derive(Debug, Parser)]
pub struct Options {
    #[clap(long)]
    payer: Option<PathBuf>,
    /// mainnet, devnet, local, default: local
    #[clap(long)]
    network: Option<String>,
    /// mainnet, testnet
    #[clap(long)]
    config: Option<String>,
}

pub async fn resize_registered_program_pda(options: Options) -> anyhow::Result<()> {
    let rpc_url = if let Some(network) = options.network {
        if network == "local" {
            String::from("http://127.0.0.1:8899")
        } else if network == "devnet" {
            String::from("https://api.devnet.solana.com")
        } else if network == "mainnet" {
            String::from("https://api.mainnet-beta.solana.com")
        } else {
            network.to_string()
        }
    } else {
        String::from("http://127.0.0.1:8899")
    };

    let mut rpc = LightClient::new(LightClientConfig {
        url: rpc_url,
        photon_url: None,
        commitment_config: None,
        fetch_active_tree: false,
        api_key: None,
    })
    .await
    .unwrap();

    let payer = if let Some(payer_path) = options.payer {
        read_keypair_file(payer_path).expect("Failed to read payer keypair")
    } else {
        let home_dir = home_dir().unwrap();
        let payer_path = home_dir.join(".config/solana/id.json");
        read_keypair_file(payer_path).expect("Failed to read payer keypair")
    };

    // Programs to resize
    let programs_to_resize = vec![
        (
            "Light System Program",
            solana_sdk::pubkey::Pubkey::from_str("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7")
                .unwrap(),
        ),
        (
            "Light Registry Program",
            solana_sdk::pubkey::Pubkey::from_str("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX")
                .unwrap(),
        ),
    ];

    for (program_name, program_id) in programs_to_resize {
        println!("Resizing registered program PDA for {}", program_name);
        println!("Program ID: {}", program_id);

        // Calculate the registered program PDA
        let registered_program_pda = solana_sdk::pubkey::Pubkey::find_program_address(
            &[program_id.to_bytes().as_slice()],
            &account_compression::ID,
        )
        .0;

        println!("Registered program PDA: {}", registered_program_pda);

        let instruction_data = account_compression::instruction::ResizeRegisteredProgramPda {};
        let accounts = account_compression::accounts::ResizeRegisteredProgramPda {
            authority: payer.pubkey(),
            registered_program_pda,
            system_program: solana_sdk::system_program::ID,
        };

        let instruction = Instruction {
            program_id: account_compression::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };

        println!("Sending resize transaction for {}...", program_name);
        match rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await
        {
            Ok(signature) => {
                println!("✓ Successfully resized {} PDA!", program_name);
                println!("  Transaction signature: {}", signature);
            }
            Err(e) => {
                println!("✗ Failed to resize {} PDA: {}", program_name, e);
                // Continue with the next program instead of failing entirely
            }
        }
        println!();
    }

    Ok(())
}
