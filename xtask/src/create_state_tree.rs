use std::path::PathBuf;

use account_compression::{NullifierQueueConfig, StateMerkleTreeConfig};
use clap::Parser;
use light_client::rpc::{RpcConnection, SolanaRpcConnection};
use light_program_test::test_env::create_state_merkle_tree_and_queue_account;
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};

#[derive(Debug, Parser)]
pub struct Options {
    #[clap(long)]
    payer: Option<PathBuf>,
    #[clap(long)]
    path: PathBuf,
    #[clap(long)]
    mt_pubkey: String,
    #[clap(long)]
    nfq_pubkey: String,
    #[clap(long)]
    cpi_pubkey: String,
    #[clap(long)]
    index: u32,
    /// mainnet, devnet, local, default: mainnet
    #[clap(long)]
    network: Option<String>,
}
pub async fn create_state_tree(options: Options) -> anyhow::Result<()> {
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
    let path = if let Some(path) = options.path.to_str() {
        String::from(path)
    } else {
        String::from("./target/state-trees/")
    };

    let mut mt_keypairs: Vec<Keypair> = vec![];
    let mut nfq_keypairs: Vec<Keypair> = vec![];
    let mut cpi_keypairs: Vec<Keypair> = vec![];

    println!(
        "ormat!( path, options.mt_pubkey) {:?}",
        format!("{}-{}.json", path, options.mt_pubkey)
    );

    let mt_keypair = read_keypair_file(format!("{}{}.json", path, options.mt_pubkey)).unwrap();
    let nfq_keypair = read_keypair_file(format!("{}{}.json", path, options.nfq_pubkey)).unwrap();
    let cpi_keypair = read_keypair_file(format!("{}{}.json", path, options.cpi_pubkey)).unwrap();
    println!("read mt: {:?}", mt_keypair.pubkey());
    println!("read nfq: {:?}", nfq_keypair.pubkey());
    println!("read cpi: {:?}", cpi_keypair.pubkey());
    mt_keypairs.push(mt_keypair);
    nfq_keypairs.push(nfq_keypair);
    cpi_keypairs.push(cpi_keypair);

    let payer = if let Some(payer) = options.payer {
        read_keypair_file(&payer).unwrap_or_else(|_| panic!("{}{}.json", path, options.cpi_pubkey))
    } else {
        read_keypair_file("/home/ananas/.config/solana/id.json")
            .expect("Default payer keypair not found in /home/ananas/.config/solana/id.json")
    };
    println!("read payer: {:?}", payer.pubkey());

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
        let tx_hash = create_state_merkle_tree_and_queue_account(
            &payer,
            true,
            &mut rpc,
            merkle_tree_keypair,
            nullifier_queue_keypair,
            Some(cpi_context_keypair),
            None,
            None,
            options.index as u64,
            &StateMerkleTreeConfig::default(),
            &NullifierQueueConfig::default(),
        )
        .await
        .unwrap();

        println!("tx_hash: {:?}", tx_hash);
    }
    Ok(())
}
