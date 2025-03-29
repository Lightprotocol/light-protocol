use std::path::PathBuf;

use clap::Parser;
use dirs::home_dir;
use light_client::rpc::{RpcConnection, SolanaRpcConnection};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
};

#[derive(Debug, Parser)]
pub struct Options {
    /// mainnet, devnet, local, default: mainnet
    #[clap(long)]
    network: Option<String>,
}

pub async fn close_state_trees(options: Options) -> anyhow::Result<()> {
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

    let payer = {
        // Construct the path to the keypair file in the user's home directory
        let keypair_path: PathBuf = home_dir()
            .expect("Could not find home directory")
            .join(".config/solana/id.json");
        read_keypair_file(keypair_path.clone())
            .unwrap_or_else(|_| panic!("Keypair not found in default path {:?}", keypair_path))
    };
    println!("read payer: {:?}", payer.pubkey());

    let balance = rpc.get_balance(&payer.pubkey()).await.unwrap();
    println!("Payer balance: {:?}", balance);
    use anchor_lang::{InstructionData, ToAccountMetas};
    use std::str::FromStr;
    let instruction_data = account_compression::instruction::CloseTrees {}.data();
    let accounts = account_compression::accounts::CloseTrees {
        fee_payer: payer.pubkey(),
        state_account_1: Pubkey::from_str("smtB1XUpt3c7j7udurMdxmAGib7RzCyBXu95fAZoHyT").unwrap(),
        nfq_1: Pubkey::from_str("nfqByCmDtLy7pkKpazApswN5H3Y4RSgCVq7NpecLHza").unwrap(),
        state_account_2: Pubkey::from_str("smtCg6rdiVANNqgZtBUzSuR5ZCcCmutiBM1WF82dA5V").unwrap(),
        nfq_2: Pubkey::from_str("nfqCyWDJhvnCchFxZyTqMMirWhnQTLUzbFdSEHSxLH9").unwrap(),
        state_account_3: Pubkey::from_str("smtd4RMDUcdvvfnjYMq3HzyyqTmgojMHAYrKd5oSHGa").unwrap(),
        nfq_3: Pubkey::from_str("nfqDgCgnkyYmDav7SCT41MHLqBVDw7ZMZ9g3FUAhKA5").unwrap(),
        state_account_4: Pubkey::from_str("smtEC1YEbkASxidPBqCvv4ZnHpiGbEoTR6jxMorukfw").unwrap(),
        nfq_4: Pubkey::from_str("nfqEqgUCSzv46UsHVCKCS4xqVpmgJGP5TbFeeNcRVTT").unwrap(),
        state_account_5: Pubkey::from_str("smtFhRnMUAzVPvK3hpqW8bdZ57EGecBZ2amgTSHDfvh").unwrap(),
        nfq_5: Pubkey::from_str("nfqFogABA4EtEauP8ti3KA96qMv6QBoGda9cTAcfKph").unwrap(),
    };
    let instruction = solana_sdk::instruction::Instruction {
        program_id: account_compression::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data,
    };
    println!("instruction {:?}", instruction);
    let tx_hash = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    println!("tx_hash: {:?}", tx_hash);

    Ok(())
}
