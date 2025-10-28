use clap::{Parser, ValueEnum};

mod bench;
mod create_batch_address_tree;
mod create_batch_state_tree;
mod create_state_tree;
mod create_update_protocol_config_ix;
mod create_vkeyrs_from_gnark_key;
mod export_photon_test_data;
mod fee;
mod fetch_accounts;
mod hash_set;
mod new_deployment;
mod print_state_tree;
mod reinit_cpi_accounts;
mod type_sizes;
mod utils;
mod zero_bytes;
mod zero_indexed_leaf;

#[derive(Debug, Clone, ValueEnum)]
enum Hash {
    Keccak,
    Poseidon,
    Sha256,
}

#[derive(Parser)]
pub struct XtaskOptions {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    GenerateZeroBytes(zero_bytes::Options),
    /// Generates a leaf of an indexed Merkle tree, for the given hash, which
    /// represents a value 0.
    GenerateZeroIndexedLeaf(zero_indexed_leaf::Options),
    /// Shows the sizes of types used as Light Protocol accounts (or their
    /// fields).
    TypeSizes,
    /// Generates the verification keys for the given gnark key.
    GenerateVkeyRs(create_vkeyrs_from_gnark_key::Options),
    /// Generates cu and heap memory usage report from a log.txt file
    Bench(bench::Options),
    /// Prints fees for different accounts.
    Fee,
    /// Hash set utilities.
    HashSet(hash_set::HashSetOptions),
    /// Create state tree
    /// Example:
    /// cargo xtask create-state-tree --mt-pubkey ./target/tree-keypairs/smtAvYA5UbTRyKAkAj5kHs1CmrA42t6WkVLi4c6mA1f.json --nfq-pubkey ./target/tree-keypairs/nfqAroCRkcZBgsAJDNkptKpsSWyM6cgB9XpWNNiCEC4.json --cpi-pubkey ./target/tree-keypairs/cpiAb2eNFf6MQeqMWEyEjSN3VJcD5hghujhmtdcMuZp.json --index 10 --network local
    CreateStateTree(create_state_tree::Options),
    ExportPhotonTestData(export_photon_test_data::Options),
    CreateBatchStateTree(create_batch_state_tree::Options),
    CreateBatchAddressTree(create_batch_address_tree::Options),
    /// cargo xtask init-new-deployment --keypairs ../light-keypairs --network local --num-foresters 3
    /// Requires program ids to be changed manually in programs.
    InitNewDeployment(new_deployment::Options),
    /// cargo xtask create-update-protocol-config --slot-length <u64>
    CreateUpdateProtocolConfigIx(create_update_protocol_config_ix::Options),
    /// Print batched state tree metadata
    /// Example:
    /// cargo xtask print-state-tree --pubkey <PUBKEY> --network mainnet
    PrintStateTree(print_state_tree::Options),
    /// Reinitialize legacy CPI context accounts to new format
    /// Example: cargo xtask reinit-cpi-accounts --network devnet
    ReinitCpiAccounts(reinit_cpi_accounts::Options),
    /// Fetch Solana accounts and save as JSON
    /// Examples:
    ///   cargo xtask fetch-accounts rpc --pubkeys 11111111111111111111111111111111 --network mainnet
    ///   cargo xtask fetch-accounts rpc --lut --pubkeys <lut_pubkey> --network mainnet
    ///   cargo xtask fetch-accounts rpc --lut --pubkeys <lut_pubkey> --add-pubkeys <pk1>,<pk2> --network mainnet
    FetchAccounts(fetch_accounts::Options),
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let opts = XtaskOptions::parse();

    match opts.command {
        Command::TypeSizes => type_sizes::type_sizes(),
        Command::GenerateZeroBytes(opts) => zero_bytes::generate_zero_bytes(opts),
        Command::GenerateZeroIndexedLeaf(opts) => {
            zero_indexed_leaf::generate_zero_indexed_leaf(opts)
        }
        Command::GenerateVkeyRs(opts) => {
            create_vkeyrs_from_gnark_key::create_vkeyrs_from_gnark_key(opts)
        }
        Command::Bench(opts) => bench::bench(opts),
        Command::Fee => fee::fees(),
        Command::HashSet(opts) => hash_set::hash_set(opts),
        Command::CreateStateTree(opts) => create_state_tree::create_state_tree(opts).await,
        Command::ExportPhotonTestData(opts) => {
            export_photon_test_data::export_photon_test_data(opts).await
        }
        Command::CreateBatchStateTree(opts) => {
            create_batch_state_tree::create_batch_state_tree(opts).await
        }
        Command::CreateBatchAddressTree(opts) => {
            create_batch_address_tree::create_batch_address_tree(opts).await
        }
        Command::InitNewDeployment(opts) => new_deployment::init_new_deployment(opts).await,
        Command::CreateUpdateProtocolConfigIx(opts) => {
            create_update_protocol_config_ix::create_update_protocol_config_ix(opts).await
        }
        Command::PrintStateTree(opts) => print_state_tree::print_state_tree(opts).await,
        Command::ReinitCpiAccounts(opts) => reinit_cpi_accounts::reinit_cpi_accounts(opts).await,
        Command::FetchAccounts(opts) => fetch_accounts::fetch_accounts(opts).await,
    }
}
