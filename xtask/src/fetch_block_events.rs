use anyhow::Result;
use clap::Parser;
use light_compressed_account::Pubkey as LightPubkey;
use light_event::parse::event_from_light_transaction;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedTransactionWithStatusMeta, TransactionDetails,
    UiInstruction, UiTransactionEncoding,
};

#[derive(Clone)]
struct ParsedInstruction {
    program_id: LightPubkey,
    data: Vec<u8>,
    accounts: Vec<LightPubkey>,
}

#[derive(Debug, Parser)]
pub struct Options {
    /// Starting slot
    #[clap(long)]
    start_slot: u64,
    /// Number of blocks to fetch (default: 10)
    #[clap(long, default_value_t = 10)]
    num_blocks: usize,
    /// Network: mainnet, devnet, testnet, local, or RPC URL
    #[clap(long, default_value = "mainnet")]
    network: String,
    /// Custom RPC URL (overrides --network)
    #[clap(long)]
    rpc_url: Option<String>,
}

fn network_to_url(network: &str) -> String {
    match network {
        "mainnet" => "https://api.mainnet-beta.solana.com".to_string(),
        "devnet" => "https://api.devnet.solana.com".to_string(),
        "testnet" => "https://api.testnet.solana.com".to_string(),
        "local" | "localnet" => "http://localhost:8899".to_string(),
        custom => custom.to_string(),
    }
}

pub async fn fetch_block_events(opts: Options) -> Result<()> {
    let rpc_url = opts
        .rpc_url
        .unwrap_or_else(|| network_to_url(&opts.network));
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let slots = client.get_blocks_with_limit(opts.start_slot, opts.num_blocks)?;

    let mut total_txs: usize = 0;
    let mut total_events: usize = 0;

    for slot in &slots {
        let config = RpcBlockConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            transaction_details: Some(TransactionDetails::Full),
            rewards: None,
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };
        let block = match client.get_block_with_config(*slot, config) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("slot {slot}: {e}");
                continue;
            }
        };

        let transactions = block.transactions.unwrap_or_default();
        let tx_count = transactions.len();
        total_txs += tx_count;

        println!("Slot {slot} -- {tx_count} transactions");

        for encoded_tx_with_meta in transactions {
            parse_and_print_tx(encoded_tx_with_meta, &mut total_events);
        }
    }

    println!(
        "\nSummary: {} blocks, {total_txs} transactions, {total_events} light events",
        slots.len()
    );

    Ok(())
}

fn parse_and_print_tx(encoded: EncodedTransactionWithStatusMeta, total_events: &mut usize) {
    let EncodedTransactionWithStatusMeta {
        transaction, meta, ..
    } = encoded;

    let versioned_tx = match transaction.decode() {
        Some(tx) => tx,
        None => return,
    };

    let sig = versioned_tx
        .signatures
        .first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let meta = match meta {
        Some(m) => m,
        None => return,
    };

    // Build full account list: static keys + loaded ALT addresses
    let mut sdk_accounts: Vec<solana_sdk::pubkey::Pubkey> =
        versioned_tx.message.static_account_keys().to_vec();

    if versioned_tx.message.address_table_lookups().is_some() {
        if let OptionSerializer::Some(loaded) = &meta.loaded_addresses {
            for addr_str in loaded.writable.iter().chain(loaded.readonly.iter()) {
                match addr_str.parse::<solana_sdk::pubkey::Pubkey>() {
                    Ok(pk) => sdk_accounts.push(pk),
                    Err(e) => {
                        eprintln!("  {sig}: bad ALT address {addr_str}: {e}");
                    }
                }
            }
        }
    }

    let accounts: Vec<LightPubkey> = sdk_accounts
        .iter()
        .map(|pk| LightPubkey::new_from_array(pk.to_bytes()))
        .collect();

    // Build inner instruction map: outer_ix_index -> [ParsedInstruction]
    let outer_count = versioned_tx.message.instructions().len();
    let mut inner_map: Vec<Vec<ParsedInstruction>> = vec![Vec::new(); outer_count];

    if let OptionSerializer::Some(inner_ixs_vec) = &meta.inner_instructions {
        for inner_ixs in inner_ixs_vec {
            let idx = inner_ixs.index as usize;
            if idx >= inner_map.len() {
                continue;
            }
            for ui_ix in &inner_ixs.instructions {
                if let UiInstruction::Compiled(c) = ui_ix {
                    let program_id = accounts[c.program_id_index as usize];
                    let data = match bs58::decode(&c.data).into_vec() {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!("  {sig}: inner ix decode error: {e}");
                            continue;
                        }
                    };
                    let ix_accounts = c.accounts.iter().map(|i| accounts[*i as usize]).collect();
                    inner_map[idx].push(ParsedInstruction {
                        program_id,
                        data,
                        accounts: ix_accounts,
                    });
                }
            }
        }
    }

    // For each instruction group (outer + its inners), call event_from_light_transaction
    let mut tx_event_count = 0usize;
    let mut tx_event_lines: Vec<String> = Vec::new();

    for (outer_idx, compiled_ix) in versioned_tx.message.instructions().iter().enumerate() {
        let mut program_ids = vec![accounts[compiled_ix.program_id_index as usize]];
        let mut data = vec![compiled_ix.data.clone()];
        let mut ix_accounts = vec![compiled_ix
            .accounts
            .iter()
            .map(|i| accounts[*i as usize])
            .collect::<Vec<_>>()];

        for ix in &inner_map[outer_idx] {
            program_ids.push(ix.program_id);
            data.push(ix.data.clone());
            ix_accounts.push(ix.accounts.clone());
        }

        match event_from_light_transaction(&program_ids, &data, ix_accounts) {
            Ok(Some(events)) => {
                for (i, event) in events.iter().enumerate() {
                    let tx_hash = bs58::encode(&event.tx_hash).into_string();
                    let inputs = event.event.input_compressed_account_hashes.len();
                    let outputs = event.event.output_compressed_account_hashes.len();
                    let new_addrs = event.new_addresses.len();
                    let compress_info = if event.event.is_compress
                        || event.event.compress_or_decompress_lamports.is_some()
                    {
                        let dir = if event.event.is_compress {
                            "compress"
                        } else {
                            "decompress"
                        };
                        if let Some(lamports) = event.event.compress_or_decompress_lamports {
                            format!(" ({dir}: {lamports} lamports)")
                        } else {
                            format!(" ({dir})")
                        }
                    } else {
                        String::new()
                    };
                    tx_event_lines.push(format!(
                        "    event[{i}] tx_hash={tx_hash}  inputs={inputs} outputs={outputs} new_addresses={new_addrs}{compress_info}"
                    ));
                }
                tx_event_count += events.len();
            }
            Ok(None) => {}
            Err(e) => eprintln!("  {sig} parse error: {e:?}"),
        }
    }

    if tx_event_count > 0 {
        println!("  {sig} -- {tx_event_count} light event(s)");
        for line in &tx_event_lines {
            println!("{line}");
        }
        *total_events += tx_event_count;
    }
}
