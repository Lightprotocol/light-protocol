use std::path::PathBuf;

use clap::Parser;
use dirs::home_dir;
use light_batched_merkle_tree::initialize_state_tree::InitStateTreeAccountsInstructionData;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use light_program_test::accounts::state_tree_v2::create_batched_state_merkle_tree;
use solana_sdk::signature::{read_keypair_file, write_keypair_file, Keypair, Signer};

#[derive(Debug, Parser)]
pub struct Options {
    #[clap(long)]
    payer: Option<PathBuf>,
    #[clap(long)]
    mt_pubkey: Option<String>,
    #[clap(long)]
    nfq_pubkey: Option<String>,
    #[clap(long)]
    cpi_pubkey: Option<String>,
    #[clap(long)]
    index: u32,
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

pub async fn create_batch_state_tree(options: Options) -> anyhow::Result<()> {
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
    let mut nfq_keypairs: Vec<Keypair> = vec![];
    let mut cpi_keypairs: Vec<Keypair> = vec![];

    if options.new {
        let mt_keypair = Keypair::new();
        let nfq_keypair = Keypair::new();
        let cpi_keypair = Keypair::new();
        println!("new mt: {:?}", mt_keypair.pubkey());
        println!("new nfq: {:?}", nfq_keypair.pubkey());
        println!("new cpi: {:?}", cpi_keypair.pubkey());

        write_keypair_file(&mt_keypair, format!("./target/mt-{}", mt_keypair.pubkey())).unwrap();
        write_keypair_file(
            &nfq_keypair,
            format!("./target/nfq-{}", nfq_keypair.pubkey()),
        )
        .unwrap();
        write_keypair_file(
            &cpi_keypair,
            format!("./target/cpi-{}", cpi_keypair.pubkey()),
        )
        .unwrap();
        mt_keypairs.push(mt_keypair);
        nfq_keypairs.push(nfq_keypair);
        cpi_keypairs.push(cpi_keypair);
    } else {
        let mt_keypair = read_keypair_file(options.mt_pubkey.unwrap()).unwrap();
        let nfq_keypair = read_keypair_file(options.nfq_pubkey.unwrap()).unwrap();
        let cpi_keypair = read_keypair_file(options.cpi_pubkey.unwrap()).unwrap();
        println!("read mt: {:?}", mt_keypair.pubkey());
        println!("read nfq: {:?}", nfq_keypair.pubkey());
        println!("read cpi: {:?}", cpi_keypair.pubkey());
        mt_keypairs.push(mt_keypair);
        nfq_keypairs.push(nfq_keypair);
        cpi_keypairs.push(cpi_keypair);
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
            InitStateTreeAccountsInstructionData::testnet_default()
        } else {
            unimplemented!()
        }
    } else {
        InitStateTreeAccountsInstructionData::default()
    };

    for ((merkle_tree_keypair, nullifier_queue_keypair), cpi_context_keypair) in mt_keypairs
        .iter()
        .zip(nfq_keypairs.iter())
        .zip(cpi_keypairs.iter())
    {
        println!(
            "creating state Merkle tree: \n\tmt {:?},\n\t nfq: {:?}\n\t cpi {:?}\n\t index {}",
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            cpi_context_keypair.pubkey(),
            options.index
        );
        let balance = rpc.get_balance(&payer.pubkey()).await.unwrap();
        println!("Payer balance: {:?}", balance);
        let tx_hash = create_batched_state_merkle_tree(
            &payer,
            true,
            &mut rpc,
            merkle_tree_keypair,
            nullifier_queue_keypair,
            cpi_context_keypair,
            config,
        )
        .await
        .unwrap();

        println!("tx_hash: {:?}", tx_hash);
    }
    Ok(())
}
