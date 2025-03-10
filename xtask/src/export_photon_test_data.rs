use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    str::FromStr,
};

use clap::Parser;
use serde_json::{json, Value};
use solana_client::{rpc_client::RpcClient, rpc_request::RpcRequest};
use solana_sdk::signature::Signature;
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};

#[derive(Debug, Parser)]
pub struct Options {
    #[clap(long)]
    test_name: String,
}

async fn export_transactions(
    address: &str,
    output_folder: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let rpc_url = "http://localhost:8899";
    let client = RpcClient::new(rpc_url.to_string());

    fs::create_dir_all(output_folder)?;

    let transactions: Value = client.send(
        RpcRequest::Custom {
            method: "getSignaturesForAddress",
        },
        json!([address]),
    )?;

    for tx in transactions.as_array().unwrap_or(&vec![]) {
        let sig = tx["signature"].as_str().unwrap_or("");
        if sig.is_empty() {
            continue;
        }

        let tx_data: EncodedConfirmedTransactionWithStatusMeta = client
            .get_transaction(
                &Signature::from_str(sig).unwrap(),
                UiTransactionEncoding::Base64,
            )
            .unwrap();

        let file_path = format!("{}/{}", output_folder, sig);
        let mut file = File::create(Path::new(&file_path))?;
        file.write_all(serde_json::to_string_pretty(&tx_data)?.as_bytes())?;
    }

    Ok(())
}

pub async fn export_photon_test_data(opt: Options) -> anyhow::Result<()> {
    let address = "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq";
    let output_folder = format!("./target/{}", opt.test_name);

    if let Err(e) = export_transactions(address, &output_folder).await {
        eprintln!("Error exporting transactions: {}", e);
    }
    Ok(())
}
