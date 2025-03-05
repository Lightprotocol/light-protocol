use std::path::PathBuf;

use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};
use clap::Parser;
use dirs::home_dir;
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
};
use light_client::rpc::{RpcConnection, SolanaRpcConnection};
use light_program_test::test_env::{initialize_accounts, EnvAccountKeypairs};
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    signature::{read_keypair_file, write_keypair_file, Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
/// cargo xtask
#[derive(Debug, Parser)]
pub struct Options {
    #[clap(long)]
    keypairs: String,
    #[clap(long)]
    network: Option<String>,
    #[clap(long, default_value = "false")]
    new: bool,
    #[clap(long)]
    payer: Option<PathBuf>,
    #[clap(long)]
    num_foresters: Option<u32>,
    #[clap(long)]
    config: Option<String>,
}

pub async fn init_new_deployment(options: Options) -> anyhow::Result<()> {
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
        String::from("https://api.mainnet-beta.solana.com")
    };
    let mut rpc = SolanaRpcConnection::new(rpc_url, None);

    let env_keypairs = EnvAccountKeypairs::new_testnet_setup();
    env_keypairs.write_to_files(&format!("{}/", options.keypairs)); // Fixed string concatenation

    let payer = if let Some(payer) = options.payer.as_ref() {
        read_keypair_file(payer).unwrap_or_else(|_| panic!("{:?}", options.payer))
    } else {
        // Construct the path to the keypair file in the user's home directory
        let keypair_path: PathBuf = home_dir()
            .expect("Could not find home directory")
            .join(".config/solana/id.json");
        read_keypair_file(keypair_path.clone())
            .unwrap_or_else(|_| panic!("Keypair not found in default path {:?}", keypair_path))
    };
    println!("read payer: {:?}", payer.pubkey());

    let (
        merkle_tree_config,
        queue_config,
        address_tree_config,
        address_queue_config,
        batched_state_tree_config,
        _batched_address_tree_config,
    ) = if let Some(config) = options.config {
        if config == "testnet" {
            (
                StateMerkleTreeConfig {
                    changelog_size: 200,
                    ..Default::default()
                },
                NullifierQueueConfig {
                    capacity: 2500,
                    ..Default::default()
                },
                AddressMerkleTreeConfig {
                    changelog_size: 200,
                    ..Default::default()
                },
                AddressQueueConfig {
                    capacity: 2500,
                    ..Default::default()
                },
                InitStateTreeAccountsInstructionData::testnet_default(),
                InitAddressTreeAccountsInstructionData::testnet_default(),
            )
        } else {
            unimplemented!("Only testnet is implemented.")
        }
    } else {
        (
            StateMerkleTreeConfig::default(),
            NullifierQueueConfig::default(),
            AddressMerkleTreeConfig::default(),
            AddressQueueConfig::default(),
            InitStateTreeAccountsInstructionData::default(),
            InitAddressTreeAccountsInstructionData::default(),
        )
    };
    {
        let transfer_instruction = system_instruction::transfer(
            &payer.pubkey(),
            &env_keypairs.governance_authority.pubkey(),
            15 * LAMPORTS_PER_SOL,
        );
        println!(
            "governance authority {}",
            env_keypairs.governance_authority.pubkey()
        );
        let latest_blockhash = rpc.get_latest_blockhash().await.unwrap();
        // Create and sign a transaction
        let transaction = Transaction::new_signed_with_payer(
            &[transfer_instruction],
            Some(&payer.pubkey()),
            &vec![&payer],
            latest_blockhash,
        );

        // Send the transaction
        rpc.process_transaction(transaction).await?;
    }
    let governance_authority = env_keypairs.governance_authority.insecure_clone();
    initialize_accounts(
        &mut rpc,
        env_keypairs,
        light_registry::protocol_config::state::ProtocolConfig::testnet_default(),
        false,
        false,
        true,
        merkle_tree_config,
        queue_config,
        address_tree_config,
        address_queue_config,
        batched_state_tree_config,
        None,
    )
    .await;
    println!("initialized accounts");

    if let Some(num_foresters) = options.num_foresters {
        for _ in 0..num_foresters {
            let forester = Keypair::new();
            println!("new forester: {:?}", forester.pubkey());

            write_keypair_file(
                &forester,
                format!("{}/forester-{}", options.keypairs, forester.pubkey()),
            )
            .unwrap();
            let ix = light_registry::sdk::create_register_forester_instruction(
                &governance_authority.pubkey(),
                &governance_authority.pubkey(),
                &forester.pubkey(),
                light_registry::ForesterConfig::default(),
            );
            rpc.create_and_send_transaction(
                &[ix],
                &governance_authority.pubkey(),
                &[&governance_authority],
            )
            .await?;
        }
    }

    Ok(())
}
