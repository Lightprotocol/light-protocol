use std::path::PathBuf;

use clap::Parser;
use dirs::home_dir;
use light_batched_merkle_tree::initialize_address_tree::InitAddressTreeAccountsInstructionData;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use light_program_test::accounts::address_tree_v2::create_batch_address_merkle_tree;
use solana_sdk::signature::{read_keypair_file, write_keypair_file, Keypair, Signer};

#[derive(Debug, Parser)]
pub struct Options {
    #[clap(long)]
    payer: Option<PathBuf>,
    #[clap(long)]
    mt_pubkey: Option<String>,
    #[clap(long)]
    /// mainnet, devnet, local, default: mainnet
    #[clap(long)]
    network: Option<String>,
    /// mainnet, devnet, local, default: mainnet
    #[clap(long, default_value = "false")]
    new: bool,
    /// mainnet, testnet
    #[clap(long)]
    config: Option<String>,
}

pub async fn create_batch_address_tree(options: Options) -> anyhow::Result<()> {
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

    let mut mt_keypairs: Vec<Keypair> = vec![];

    if options.new {
        let mt_keypair = Keypair::new();
        println!("new mt: {:?}", mt_keypair.pubkey());

        write_keypair_file(&mt_keypair, format!("./target/mt-{}", mt_keypair.pubkey())).unwrap();

        mt_keypairs.push(mt_keypair);
    } else {
        let mt_keypair = read_keypair_file(options.mt_pubkey.unwrap()).unwrap();
        println!("read mt: {:?}", mt_keypair.pubkey());
        mt_keypairs.push(mt_keypair);
    }

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

    let config = if let Some(config) = options.config {
        if config == "testnet" {
            InitAddressTreeAccountsInstructionData::testnet_default()
        } else {
            unimplemented!()
        }
    } else {
        InitAddressTreeAccountsInstructionData::default()
    };

    for merkle_tree_keypair in mt_keypairs.iter() {
        println!(
            "creating address Merkle tree: \n\tmt {:?}",
            merkle_tree_keypair.pubkey(),
        );
        let balance = rpc.get_balance(&payer.pubkey()).await.unwrap();
        println!("Payer balance: {:?}", balance);
        let tx_hash =
            create_batch_address_merkle_tree(&mut rpc, &payer, merkle_tree_keypair, config)
                .await
                .unwrap();

        println!("tx_hash: {:?}", tx_hash);
    }
    Ok(())
}
