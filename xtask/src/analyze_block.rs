use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::Parser;
use light_instruction_decoder::{DecodedField, DecoderRegistry};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedTransactionWithStatusMeta, TransactionDetails,
    UiConfirmedBlock, UiInstruction, UiTransactionEncoding,
};
use tabled::{settings::Style, Table, Tabled};

const ACCOUNT_COMPRESSION_PROGRAM_ID: &str = "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq";

#[derive(Debug, Parser)]
pub struct Options {
    /// Block slot to fetch and analyze.
    #[clap(long)]
    slot: u64,
    /// Filter to transactions that touch this program id (default: account-compression).
    #[clap(long, default_value = ACCOUNT_COMPRESSION_PROGRAM_ID)]
    program_id: String,
    /// Also decode inner instructions whose program id is not the filter
    /// program. Top-level instructions of matching txs are always decoded.
    #[clap(long)]
    all_instructions: bool,
    /// Network: mainnet, devnet, testnet, local, or full RPC URL.
    #[clap(long, default_value = "mainnet")]
    network: String,
    /// Custom RPC URL (overrides --network).
    #[clap(long)]
    rpc_url: Option<String>,
    /// Bypass the on-disk block cache and force a fresh RPC fetch.
    #[clap(long)]
    no_cache: bool,
    /// Render a compact table instead of per-instruction verbose output.
    #[clap(long)]
    table: bool,
}

fn cache_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target")
        .join("analyze-block-cache")
}

fn cache_path(slot: u64) -> PathBuf {
    cache_dir().join(format!("{slot}.json"))
}

fn load_or_fetch_block(
    slot: u64,
    no_cache: bool,
    network: &str,
    rpc_url: Option<&str>,
) -> Result<UiConfirmedBlock> {
    let path = cache_path(slot);
    if !no_cache && path.is_file() {
        let bytes = fs::read(&path)
            .with_context(|| format!("read cache {}", path.display()))?;
        let block: UiConfirmedBlock = serde_json::from_slice(&bytes)
            .with_context(|| format!("deserialize cache {}", path.display()))?;
        eprintln!("cache hit: {}", path.display());
        return Ok(block);
    }

    let url = rpc_url
        .map(str::to_string)
        .unwrap_or_else(|| network_to_url(network));
    let client = RpcClient::new_with_commitment(url, CommitmentConfig::confirmed());
    let config = RpcBlockConfig {
        encoding: Some(UiTransactionEncoding::Base64),
        transaction_details: Some(TransactionDetails::Full),
        rewards: Some(false),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };
    let block = client
        .get_block_with_config(slot, config)
        .with_context(|| format!("get_block slot={slot}"))?;

    if let Err(e) = (|| -> Result<()> {
        fs::create_dir_all(cache_dir())?;
        let bytes = serde_json::to_vec(&block)?;
        fs::write(&path, bytes)?;
        eprintln!("cached block to {}", path.display());
        Ok(())
    })() {
        eprintln!("warning: failed to write cache {}: {e:#}", path.display());
    }

    Ok(block)
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

#[derive(Default)]
struct Stats {
    total_txs: usize,
    matching_txs: usize,
    decoded_ix: usize,
    undecoded_ix: usize,
    by_instruction: BTreeMap<String, usize>,
}

pub async fn analyze_block(opts: Options) -> Result<()> {
    let filter_program: Pubkey = opts.program_id.parse().context("invalid --program-id")?;
    let mut block = load_or_fetch_block(opts.slot, opts.no_cache, &opts.network, opts.rpc_url.as_deref())?;
    let txs = block.transactions.take().unwrap_or_default();
    let total_txs = txs.len();

    if opts.table {
        return render_table(&opts, &block, txs, total_txs, &filter_program);
    }

    let registry = DecoderRegistry::new();
    let mut stats = Stats {
        total_txs,
        ..Stats::default()
    };

    println!(
        "Slot {} -- {} transactions, blockhash {} parent {}",
        opts.slot, stats.total_txs, block.blockhash, block.parent_slot
    );
    println!("Filter program: {filter_program}");
    println!();

    for encoded in txs {
        process_tx(
            encoded,
            &filter_program,
            opts.all_instructions,
            &registry,
            &mut stats,
        );
    }

    println!("=== Summary ===");
    println!("Transactions in block       : {}", stats.total_txs);
    println!("Transactions matching filter: {}", stats.matching_txs);
    println!("Decoded instructions        : {}", stats.decoded_ix);
    println!("Undecoded instructions      : {}", stats.undecoded_ix);
    if !stats.by_instruction.is_empty() {
        let mut entries: Vec<_> = stats.by_instruction.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        println!("\nInstruction histogram:");
        for (name, count) in entries {
            println!("  {count:>5}  {name}");
        }
    }

    Ok(())
}

const SYSTEM_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");

struct Row {
    sig: String,
    ok: bool,
    cu: Option<u64>,
    fee: u64,
    in_smt: Option<Pubkey>,
    nfq: Option<Pubkey>,
    out_smt: Option<Pubkey>,
    rent: Vec<u64>,
    top_ix: Vec<String>,
    inner_ix: Vec<String>,
}

#[derive(Tabled)]
struct RowDisplay {
    sig: String,
    st: &'static str,
    cu: String,
    fee: String,
    in_smt: String,
    nfq: String,
    out_smt: String,
    rent_lamports: String,
}

impl From<&Row> for RowDisplay {
    fn from(r: &Row) -> Self {
        let sig: String = r.sig.chars().take(8).collect();
        let cu = r.cu.map(nfmt).unwrap_or_else(|| "-".into());
        let rent = if r.rent.is_empty() {
            "-".to_string()
        } else {
            r.rent
                .iter()
                .map(|l| nfmt(*l))
                .collect::<Vec<_>>()
                .join("+")
        };
        Self {
            sig,
            st: if r.ok { "ok" } else { "err" },
            cu,
            fee: nfmt(r.fee),
            in_smt: r.in_smt.as_ref().map(short).unwrap_or_else(|| "-".into()),
            nfq: r.nfq.as_ref().map(short).unwrap_or_else(|| "-".into()),
            out_smt: r.out_smt.as_ref().map(short).unwrap_or_else(|| "-".into()),
            rent_lamports: rent,
        }
    }
}

#[derive(Tabled)]
struct AccountUsage {
    pubkey: String,
    in_smt: String,
    nfq: String,
    out_smt: String,
}

#[derive(Tabled)]
struct CuStats {
    metric: &'static str,
    total: String,
    min: String,
    avg: String,
    max: String,
}

#[derive(Tabled)]
struct RentStats {
    metric: &'static str,
    transfers: String,
    total_lamports: String,
}

#[derive(Tabled)]
struct IxSummary {
    instruction: String,
    top: String,
    inner: String,
    total: String,
}

fn short(pk: &Pubkey) -> String {
    let s = pk.to_string();
    s.chars().take(4).collect()
}

fn nfmt(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(b as char);
    }
    out
}

fn label_for(
    registry: &DecoderRegistry,
    program_id: &Pubkey,
    data: &[u8],
    accounts: &[Pubkey],
    ix_account_indices: &[u8],
) -> String {
    let metas: Vec<light_instruction_decoder::solana_instruction::AccountMeta> = ix_account_indices
        .iter()
        .filter_map(|i| accounts.get(*i as usize).copied())
        .map(|pk| {
            light_instruction_decoder::solana_instruction::AccountMeta::new_readonly(
                light_instruction_decoder::solana_pubkey::Pubkey::new_from_array(pk.to_bytes()),
                false,
            )
        })
        .collect();
    let dpk = light_instruction_decoder::solana_pubkey::Pubkey::new_from_array(program_id.to_bytes());
    match registry.decode(&dpk, data, &metas) {
        Some((decoded, decoder)) => format!("{}::{}", decoder.program_name(), decoded.name),
        None => format!("{program_id} <undecoded>"),
    }
}

fn collect_row(
    encoded: &EncodedTransactionWithStatusMeta,
    filter_program: &Pubkey,
    registry: &DecoderRegistry,
) -> Option<Row> {
    let versioned_tx = encoded.transaction.decode()?;
    let meta = encoded.meta.as_ref()?;

    let mut accounts: Vec<Pubkey> = versioned_tx.message.static_account_keys().to_vec();
    if versioned_tx.message.address_table_lookups().is_some() {
        if let OptionSerializer::Some(loaded) = &meta.loaded_addresses {
            for s in loaded.writable.iter().chain(loaded.readonly.iter()) {
                if let Ok(pk) = s.parse::<Pubkey>() {
                    accounts.push(pk);
                }
            }
        }
    }
    if !accounts.contains(filter_program) {
        return None;
    }

    let mut row = Row {
        sig: versioned_tx
            .signatures
            .first()
            .map(|s| s.to_string())
            .unwrap_or_default(),
        ok: meta.err.is_none(),
        cu: match &meta.compute_units_consumed {
            OptionSerializer::Some(v) => Some(*v),
            _ => None,
        },
        fee: meta.fee,
        in_smt: None,
        nfq: None,
        out_smt: None,
        rent: Vec::new(),
        top_ix: Vec::new(),
        inner_ix: Vec::new(),
    };

    for ix in versioned_tx.message.instructions() {
        let prog = match accounts.get(ix.program_id_index as usize) {
            Some(p) => *p,
            None => continue,
        };
        row.top_ix
            .push(label_for(registry, &prog, &ix.data, &accounts, &ix.accounts));
    }

    if let OptionSerializer::Some(inner_groups) = &meta.inner_instructions {
        for group in inner_groups {
            for ui_ix in &group.instructions {
                let UiInstruction::Compiled(c) = ui_ix else { continue };
                let prog = match accounts.get(c.program_id_index as usize) {
                    Some(p) => *p,
                    None => continue,
                };
                let data = match bs58::decode(&c.data).into_vec() {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                row.inner_ix
                    .push(label_for(registry, &prog, &data, &accounts, &c.accounts));

                if prog == *filter_program && row.out_smt.is_none() && row.in_smt.is_none() {
                    let pk_at = |i: usize| {
                        c.accounts
                            .get(i)
                            .and_then(|idx| accounts.get(*idx as usize).copied())
                    };
                    // CPI account layout (system -> account-compression):
                    //   0: authority
                    //   1: registered_program_pda
                    //   2..: outputs first (new leaves' tree, then its queue),
                    //        inputs after (nullifier queue, then input tree).
                    // For 5-acct calls (append + nullify):
                    //   pos 2 = output tree, pos 3 = input nfq, pos 4 = input tree.
                    // For 4-acct calls (append-only, no nullifications):
                    //   pos 2 = output tree, pos 3 = output queue.
                    row.out_smt = pk_at(2);
                    row.nfq = pk_at(3);
                    row.in_smt = pk_at(4);
                } else if prog == SYSTEM_PROGRAM_ID && data.len() == 12 {
                    let disc = u32::from_le_bytes(data[0..4].try_into().unwrap());
                    if disc == 2 {
                        let lamports = u64::from_le_bytes(data[4..12].try_into().unwrap());
                        row.rent.push(lamports);
                    }
                }
            }
        }
    }

    Some(row)
}

fn render_table(
    opts: &Options,
    block: &UiConfirmedBlock,
    txs: Vec<EncodedTransactionWithStatusMeta>,
    total_txs: usize,
    filter_program: &Pubkey,
) -> Result<()> {
    let registry = DecoderRegistry::new();
    let rows: Vec<Row> = txs
        .iter()
        .filter_map(|e| collect_row(e, filter_program, &registry))
        .collect();

    println!(
        "Slot {} -- {} txs total, {} match filter {}",
        opts.slot,
        total_txs,
        rows.len(),
        filter_program
    );
    println!("blockhash {}  parent {}", block.blockhash, block.parent_slot);
    println!();

    let display_rows: Vec<RowDisplay> = rows.iter().map(RowDisplay::from).collect();
    let mut table = Table::new(&display_rows);
    table.with(Style::sharp());
    println!("{table}");
    println!();

    // Account usage table: one row per distinct pubkey appearing as in_smt / nfq / out_smt.
    fn count_pos<F: Fn(&Row) -> Option<Pubkey>>(rows: &[Row], f: F) -> BTreeMap<String, usize> {
        let mut m: BTreeMap<String, usize> = BTreeMap::new();
        for r in rows {
            let key = f(r).map(|pk| short(&pk)).unwrap_or_else(|| "-".into());
            *m.entry(key).or_insert(0) += 1;
        }
        m
    }
    let in_counts = count_pos(&rows, |r| r.in_smt);
    let nfq_counts = count_pos(&rows, |r| r.nfq);
    let out_counts = count_pos(&rows, |r| r.out_smt);
    let mut keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    keys.extend(in_counts.keys().cloned());
    keys.extend(nfq_counts.keys().cloned());
    keys.extend(out_counts.keys().cloned());
    let cell = |n: usize| {
        if n == 0 {
            String::new()
        } else {
            nfmt(n as u64)
        }
    };
    let usage: Vec<AccountUsage> = keys
        .iter()
        .map(|k| AccountUsage {
            pubkey: k.clone(),
            in_smt: cell(*in_counts.get(k).unwrap_or(&0)),
            nfq: cell(*nfq_counts.get(k).unwrap_or(&0)),
            out_smt: cell(*out_counts.get(k).unwrap_or(&0)),
        })
        .collect();
    println!("Account usage:");
    let mut t = Table::new(&usage);
    t.with(Style::sharp());
    println!("{t}");
    println!();

    // CU stats table.
    let cus: Vec<u64> = rows.iter().filter_map(|r| r.cu).collect();
    if !cus.is_empty() {
        let min = *cus.iter().min().unwrap();
        let max = *cus.iter().max().unwrap();
        let total: u64 = cus.iter().sum();
        let avg = total / cus.len() as u64;
        let mut t = Table::new([CuStats {
            metric: "cu",
            total: nfmt(total),
            min: nfmt(min),
            avg: nfmt(avg),
            max: nfmt(max),
        }]);
        t.with(Style::sharp());
        println!("Compute units:");
        println!("{t}");
        println!();
    }

    // Rent stats table.
    let rents: Vec<u64> = rows.iter().flat_map(|r| r.rent.iter().copied()).collect();
    if !rents.is_empty() {
        let total: u64 = rents.iter().sum();
        let mut t = Table::new([RentStats {
            metric: "rent",
            transfers: nfmt(rents.len() as u64),
            total_lamports: nfmt(total),
        }]);
        t.with(Style::sharp());
        println!("Rent transfers:");
        println!("{t}");
        println!();
    }

    // Instruction summary table: top vs inner counts per <program>::<ix>.
    let mut top_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut inner_counts: BTreeMap<String, usize> = BTreeMap::new();
    for r in &rows {
        for l in &r.top_ix {
            *top_counts.entry(l.clone()).or_insert(0) += 1;
        }
        for l in &r.inner_ix {
            *inner_counts.entry(l.clone()).or_insert(0) += 1;
        }
    }
    let mut all: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    all.extend(top_counts.keys().cloned());
    all.extend(inner_counts.keys().cloned());
    let mut summary_raw: Vec<(String, usize, usize)> = all
        .into_iter()
        .map(|k| {
            let top = *top_counts.get(&k).unwrap_or(&0);
            let inner = *inner_counts.get(&k).unwrap_or(&0);
            (k, top, inner)
        })
        .collect();
    summary_raw.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(&b.0)));
    let summary: Vec<IxSummary> = summary_raw
        .into_iter()
        .map(|(k, top, inner)| IxSummary {
            instruction: k,
            top: nfmt(top as u64),
            inner: nfmt(inner as u64),
            total: nfmt((top + inner) as u64),
        })
        .collect();
    println!("Instruction summary:");
    let mut t = Table::new(&summary);
    t.with(Style::sharp());
    println!("{t}");

    Ok(())
}

fn process_tx(
    encoded: EncodedTransactionWithStatusMeta,
    filter_program: &Pubkey,
    all_instructions: bool,
    registry: &DecoderRegistry,
    stats: &mut Stats,
) {
    let EncodedTransactionWithStatusMeta {
        transaction, meta, ..
    } = encoded;
    let Some(versioned_tx) = transaction.decode() else {
        return;
    };
    let Some(meta) = meta else {
        return;
    };

    let mut accounts: Vec<Pubkey> = versioned_tx.message.static_account_keys().to_vec();
    if versioned_tx.message.address_table_lookups().is_some() {
        if let OptionSerializer::Some(loaded) = &meta.loaded_addresses {
            for s in loaded.writable.iter().chain(loaded.readonly.iter()) {
                if let Ok(pk) = s.parse::<Pubkey>() {
                    accounts.push(pk);
                }
            }
        }
    }

    if !accounts.contains(filter_program) {
        return;
    }
    stats.matching_txs += 1;

    let sig = versioned_tx
        .signatures
        .first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let success = meta.err.is_none();
    let cu_str = match &meta.compute_units_consumed {
        OptionSerializer::Some(v) => v.to_string(),
        _ => "-".to_string(),
    };
    println!(
        "tx {sig}  status={}  fee={}  cu={cu_str}",
        if success { "ok" } else { "err" },
        meta.fee
    );

    for (outer_idx, ix) in versioned_tx.message.instructions().iter().enumerate() {
        decode_one(
            registry,
            &accounts,
            ix.program_id_index as usize,
            &ix.accounts,
            &ix.data,
            outer_idx,
            None,
            filter_program,
            all_instructions,
            stats,
        );
    }

    if let OptionSerializer::Some(inner_groups) = &meta.inner_instructions {
        for group in inner_groups {
            let outer_idx = group.index as usize;
            for (inner_idx, ui_ix) in group.instructions.iter().enumerate() {
                if let UiInstruction::Compiled(c) = ui_ix {
                    let Ok(data) = bs58::decode(&c.data).into_vec() else {
                        continue;
                    };
                    decode_one(
                        registry,
                        &accounts,
                        c.program_id_index as usize,
                        &c.accounts,
                        &data,
                        outer_idx,
                        Some(inner_idx),
                        filter_program,
                        all_instructions,
                        stats,
                    );
                }
            }
        }
    }

    println!();
}

#[allow(clippy::too_many_arguments)]
fn decode_one(
    registry: &DecoderRegistry,
    accounts: &[Pubkey],
    program_id_index: usize,
    account_indices: &[u8],
    data: &[u8],
    outer_idx: usize,
    inner_idx: Option<usize>,
    filter_program: &Pubkey,
    all_instructions: bool,
    stats: &mut Stats,
) {
    let Some(program_id) = accounts.get(program_id_index).copied() else {
        return;
    };
    let is_top = inner_idx.is_none();
    if !is_top && !all_instructions && program_id != *filter_program {
        return;
    }

    let metas: Vec<light_instruction_decoder::solana_instruction::AccountMeta> = account_indices
        .iter()
        .filter_map(|&i| accounts.get(i as usize).copied())
        .map(|pk| {
            light_instruction_decoder::solana_instruction::AccountMeta::new_readonly(
                light_instruction_decoder::solana_pubkey::Pubkey::new_from_array(pk.to_bytes()),
                false,
            )
        })
        .collect();

    let prefix = match inner_idx {
        Some(i) => format!("  [{outer_idx}.{i}] inner "),
        None => format!("  [{outer_idx}]   top   "),
    };

    let dpk = light_instruction_decoder::solana_pubkey::Pubkey::new_from_array(program_id.to_bytes());
    match registry.decode(&dpk, data, &metas) {
        Some((decoded, decoder)) => {
            stats.decoded_ix += 1;
            *stats
                .by_instruction
                .entry(format!("{}::{}", decoder.program_name(), decoded.name))
                .or_insert(0) += 1;
            println!(
                "{prefix}{} :: {} ({} bytes, {} accts)",
                decoder.program_name(),
                decoded.name,
                data.len(),
                metas.len()
            );
            print_fields(&decoded.fields, 4);
            print_accounts(&decoded.account_names, account_indices, accounts, 4);
        }
        None => {
            stats.undecoded_ix += 1;
            let disc = data.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>();
            println!(
                "{prefix}{program_id} <undecoded>  disc=0x{disc} ({} bytes, {} accts)",
                data.len(),
                metas.len()
            );
        }
    }
}

fn print_fields(fields: &[DecodedField], indent: usize) {
    for f in fields {
        let pad = " ".repeat(indent);
        if f.children.is_empty() {
            println!("{pad}{}: {}", f.name, f.value);
        } else {
            println!("{pad}{}:", f.name);
            print_fields(&f.children, indent + 2);
        }
    }
}

fn print_accounts(
    account_names: &[String],
    account_indices: &[u8],
    accounts: &[Pubkey],
    indent: usize,
) {
    if account_indices.is_empty() {
        return;
    }
    let pad = " ".repeat(indent);
    println!("{pad}accounts:");
    for (i, idx) in account_indices.iter().enumerate() {
        let name = account_names
            .get(i)
            .cloned()
            .unwrap_or_else(|| format!("#{i}"));
        match accounts.get(*idx as usize) {
            Some(pk) => println!("{pad}  {i:>2} {name}: {pk}"),
            None => println!("{pad}  {i:>2} {name}: <oob index {idx}>"),
        }
    }
}
