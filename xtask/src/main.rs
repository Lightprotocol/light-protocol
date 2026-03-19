use clap::{Parser, ValueEnum};

mod bench;
mod close_buffer;
mod create_batch_address_tree;
mod create_batch_state_tree;
mod create_compressible_config;
mod create_ctoken_account;
mod create_state_tree;
mod create_update_protocol_config_ix;
mod create_vkeyrs_from_gnark_key;
mod export_photon_test_data;
mod fee;
mod fetch_accounts;
mod fetch_block_events;
mod fetch_failed_txs;
mod fetch_keypair_txs;
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
    /// Fetch failed transactions from Light Registry program
    /// Examples:
    ///   cargo xtask fetch-failed-txs --minutes 10 --network mainnet
    ///   cargo xtask fetch-failed-txs --minutes 30 --network devnet
    FetchFailedTxs(fetch_failed_txs::Options),
    /// Fetch the last N blocks from a start slot and parse Light Protocol events
    /// Example: cargo xtask fetch-block-events --start-slot 300000000 --network mainnet
    FetchBlockEvents(fetch_block_events::Options),
    /// Count transactions per time bucket for a list of addresses
    /// Example: cargo xtask fetch-keypair-txs 8GDc4p3fpbxJZmpZB3Lx3yN1984XS2HVnMi7J7rTyeC7 3PrXqmhEcgPo2a5aTtCTYzgmuXRSx5imbUTDkz6SZMun --minutes 10 --buckets 6 --network mainnet
    FetchKeypairTxs(fetch_keypair_txs::Options),
    /// Create compressible config (config counter + compressible config)
    /// Creates the config counter PDA and a compressible config with default RentConfig.
    /// Example: cargo xtask create-compressible-config --network devnet
    CreateCompressibleConfig(create_compressible_config::Options),
    /// Create a compressible cToken account with default config
    /// Example: cargo xtask create-ctoken-account --network devnet
    /// Example with existing mint: cargo xtask create-ctoken-account --mint <MINT_PUBKEY> --network devnet
    CreateCtokenAccount(create_ctoken_account::Options),
    /// Close a BPF Upgradeable Loader buffer account via Squads multisig.
    /// Serializes the Close instruction as a bs58 message for the Squads TX builder.
    /// Example: cargo xtask close-buffer --buffer FMkzXMexKDUKGxAm7oGsjs4LGEMhzk9C6uuYJBwJbjiN
    CloseBuffer(close_buffer::Options),
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
        Command::FetchFailedTxs(opts) => fetch_failed_txs::fetch_failed_txs(opts).await,
        Command::FetchBlockEvents(opts) => fetch_block_events::fetch_block_events(opts).await,
        Command::FetchKeypairTxs(opts) => fetch_keypair_txs::fetch_keypair_txs(opts).await,
        Command::CreateCompressibleConfig(opts) => {
            create_compressible_config::create_compressible_config(opts).await
        }
        Command::CreateCtokenAccount(opts) => {
            create_ctoken_account::create_ctoken_account(opts).await
        }
        Command::CloseBuffer(opts) => close_buffer::close_buffer(opts),
    }
}
