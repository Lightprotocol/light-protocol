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
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use light_program_test::{
    accounts::{initialize::initialize_accounts, test_keypairs::TestKeypairs},
    ProgramTestConfig,
};
#[allow(deprecated)]
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
    #[clap(long)]
    num_v2_trees: Option<u32>,
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
    let mut rpc = LightClient::new(LightClientConfig {
        url: rpc_url,
        photon_url: None,
        commitment_config: None,
        fetch_active_tree: false,
        api_key: None,
    })
    .await
    .unwrap();

    let test_keypairs = new_testnet_setup();
    write_to_files(&test_keypairs, &format!("{}/", options.keypairs));

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

    let mut test_keypairs = new_testnet_setup();
    test_keypairs.governance_authority = payer.insecure_clone();
    write_to_files(&test_keypairs, &format!("{}/", options.keypairs));
    println!("forester {:?}", test_keypairs.forester.pubkey());
    let transfer_instruction = system_instruction::transfer(
        &payer.pubkey(),
        &test_keypairs.forester.pubkey(),
        1_000_000_000,
    );
    let latest_blockhash = rpc.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&payer.pubkey()),
        &vec![&payer],
        latest_blockhash.0,
    );
    rpc.process_transaction_with_context(transaction).await?;
    let (
        _merkle_tree_config,
        _queue_config,
        _address_tree_config,
        _address_queue_config,
        _batched_state_tree_config,
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
            &test_keypairs.governance_authority.pubkey(),
            15 * LAMPORTS_PER_SOL,
        );
        println!(
            "governance authority {}",
            test_keypairs.governance_authority.pubkey()
        );
        let latest_blockhash = rpc.get_latest_blockhash().await.unwrap();
        // Create and sign a transaction
        let transaction = Transaction::new_signed_with_payer(
            &[transfer_instruction],
            Some(&payer.pubkey()),
            &vec![&payer],
            latest_blockhash.0,
        );

        // Send the transaction
        rpc.process_transaction(transaction).await?;
    }
    let governance_authority = test_keypairs.governance_authority.insecure_clone();
    let config = ProgramTestConfig {
        protocol_config: light_registry::protocol_config::state::ProtocolConfig::testnet_default(),
        with_prover: false,
        additional_programs: None,
        skip_second_v1_tree: true,
        v1_state_tree_config: StateMerkleTreeConfig::default(),
        v2_state_tree_config: Some(InitStateTreeAccountsInstructionData::default()),
        v2_address_tree_config: Some(InitAddressTreeAccountsInstructionData::default()),
        ..Default::default()
    };
    initialize_accounts(&mut rpc, &config, &test_keypairs)
        .await
        .unwrap();
    println!("initialized accounts");

    if let Some(num_foresters) = options.num_foresters {
        for _ in 0..num_foresters {
            let forester = Keypair::new();
            println!("new forester: {:?}", forester.pubkey());

            write_keypair_file(
                &forester,
                format!("{}/forester-{}.json", options.keypairs, forester.pubkey()),
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

    if let Some(num_v2_trees) = options.num_v2_trees {
        for i in 1..=num_v2_trees {
            let prefix = options.keypairs.clone();
            let v2_state_tree = Keypair::new();
            println!("new v2_state_tree: {:?}", v2_state_tree.pubkey());
            let v2_state_tree_queue = Keypair::new();
            println!(
                "new v2_state_tree_queue: {:?}",
                v2_state_tree_queue.pubkey()
            );
            let v2_state_tree_cpi = Keypair::new();
            println!("new v2_state_tree_cpi: {:?}", v2_state_tree_cpi.pubkey());

            let v2_address_mt = Keypair::new();
            println!("new v2_address_mt: {:?}", v2_address_mt.pubkey());

            write_keypair_file(
                &v2_state_tree,
                format!(
                    "{}batched-state{}_{}.json",
                    prefix,
                    i,
                    v2_state_tree.pubkey()
                ),
            )
            .unwrap();
            write_keypair_file(
                &v2_state_tree_queue,
                format!(
                    "{}batched-state/batched_output_queue_{}_{}.json",
                    prefix,
                    i,
                    v2_state_tree_queue.pubkey()
                ),
            )
            .unwrap();
            write_keypair_file(
                &v2_state_tree_cpi,
                format!(
                    "{}batched_cpi_context_{}_{}.json",
                    prefix,
                    i,
                    v2_state_tree_cpi.pubkey()
                ),
            )
            .unwrap();
            write_keypair_file(
                &v2_address_mt,
                format!(
                    "{}batched_amt_{}_{}.json",
                    prefix,
                    i,
                    v2_address_mt.pubkey()
                ),
            )
            .unwrap();
            light_program_test::accounts::state_tree_v2::create_batched_state_merkle_tree(
                &test_keypairs.governance_authority,
                true,
                &mut rpc,
                &v2_state_tree,
                &v2_state_tree_queue,
                &v2_state_tree_cpi,
                config.v2_state_tree_config.unwrap(),
            )
            .await?;
            light_program_test::accounts::address_tree_v2::create_batch_address_merkle_tree(
                &mut rpc,
                &test_keypairs.governance_authority,
                &v2_address_mt,
                config.v2_address_tree_config.unwrap(),
            )
            .await?;
        }
    }

    Ok(())
}

pub fn new_testnet_setup() -> TestKeypairs {
    let prefix = String::from("target/deploy/");
    let state_merkle_tree = Keypair::new();
    let nullifier_queue = Keypair::new();
    let governance_authority = Keypair::new();
    let forester = Keypair::new();
    let address_merkle_tree = Keypair::new();
    let address_merkle_tree_queue = Keypair::new();
    let cpi_context_account = Keypair::new();
    let system_program =
        read_keypair_file(format!("{}light_compressed_token-keypair.json", prefix)).unwrap();
    let registry_program =
        read_keypair_file(format!("{}light_registry-keypair.json", prefix)).unwrap();
    TestKeypairs {
        state_merkle_tree,
        nullifier_queue,
        governance_authority,
        forester,
        address_merkle_tree,
        address_merkle_tree_queue,
        cpi_context_account,
        system_program,
        registry_program,
        batched_state_merkle_tree: Keypair::new(),
        batched_output_queue: Keypair::new(),
        batched_cpi_context: Keypair::new(),
        batch_address_merkle_tree: Keypair::new(),
        state_merkle_tree_2: Keypair::new(),
        nullifier_queue_2: Keypair::new(),
        cpi_context_2: Keypair::new(),
        group_pda_seed: Keypair::new(),
    }
}

/// Write all keypairs to files
pub fn write_to_files(keypairs: &TestKeypairs, prefix: &str) {
    write_keypair_file(
        &keypairs.batched_state_merkle_tree,
        format!(
            "{}batched-state{}.json",
            prefix,
            keypairs.batched_state_merkle_tree.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.state_merkle_tree,
        format!(
            "{}smt1_{}.json",
            prefix,
            keypairs.state_merkle_tree.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.nullifier_queue,
        format!("{}nfq1_{}.json", prefix, keypairs.nullifier_queue.pubkey()),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.governance_authority,
        format!(
            "{}ga1_{}.json",
            prefix,
            keypairs.governance_authority.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.forester,
        format!("{}forester_{}.json", prefix, keypairs.forester.pubkey()),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.address_merkle_tree,
        format!(
            "{}amt1_{}.json",
            prefix,
            keypairs.address_merkle_tree.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.address_merkle_tree_queue,
        format!(
            "{}aq1_{}.json",
            prefix,
            keypairs.address_merkle_tree_queue.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.cpi_context_account,
        format!(
            "{}cpi1_{}.json",
            prefix,
            keypairs.cpi_context_account.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.system_program,
        format!("{}system_{}.json", prefix, keypairs.system_program.pubkey()),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.registry_program,
        format!(
            "{}registry_{}.json",
            prefix,
            keypairs.registry_program.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.batched_output_queue,
        format!(
            "{}batched-state/batched_output_queue_{}.json",
            prefix,
            keypairs.batched_output_queue.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.batched_cpi_context,
        format!(
            "{}batched_cpi_context_{}.json",
            prefix,
            keypairs.batched_cpi_context.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.batch_address_merkle_tree,
        format!(
            "{}batched_amt1_{}.json",
            prefix,
            keypairs.batch_address_merkle_tree.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.state_merkle_tree_2,
        format!(
            "{}smt2_{}.json",
            prefix,
            keypairs.state_merkle_tree_2.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.nullifier_queue_2,
        format!(
            "{}nfq2_{}.json",
            prefix,
            keypairs.nullifier_queue_2.pubkey()
        ),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.cpi_context_2,
        format!("{}cpi2_{}.json", prefix, keypairs.cpi_context_2.pubkey()),
    )
    .unwrap();
    write_keypair_file(
        &keypairs.group_pda_seed,
        format!(
            "{}group_pda_seed_{}.json",
            prefix,
            keypairs.group_pda_seed.pubkey()
        ),
    )
    .unwrap();
}
