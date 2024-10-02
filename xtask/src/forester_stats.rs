use account_compression::{AddressMerkleTreeAccount, QueueAccount, StateMerkleTreeAccount};
use anchor_lang::{AccountDeserialize, Discriminator};
use clap::Parser;
use light_concurrent_merkle_tree::copy::ConcurrentMerkleTreeCopy;
use light_hash_set::HashSet;
use light_hasher::Poseidon;
use light_registry::{protocol_config::state::ProtocolConfigPda, EpochPda, ForesterEpochPda};
use solana_sdk::{account::ReadableAccount, commitment_config::CommitmentConfig};
#[derive(Debug, Parser)]
pub struct Options {
    /// Select to run compressed token program tests.
    #[clap(long)]
    full: bool,
    #[clap(long)]
    protocol_config: bool,
    #[clap(long, default_value_t = true)]
    queue: bool,
}

pub fn fetch_foreter_stats(opts: Options) -> anyhow::Result<()> {
    let commitment_config = CommitmentConfig::confirmed();
    let rpc_url = std::env::var("RPC_URL")
        .expect("RPC_URL environment variable not set, export RPC_URL=<url>");

    let client =
        solana_client::rpc_client::RpcClient::new_with_commitment(rpc_url, commitment_config);
    let registry_accounts = client
        .get_program_accounts(&light_registry::ID)
        .expect("Failed to fetch accounts for registry program.");

    let mut forester_epoch_pdas = vec![];
    let mut epoch_pdas = vec![];
    let mut protocol_config_pdas = vec![];
    for (_, account) in registry_accounts {
        match account.data()[0..8].try_into().unwrap() {
            ForesterEpochPda::DISCRIMINATOR => {
                let forester_epoch_pda =
                    ForesterEpochPda::try_deserialize_unchecked(&mut account.data())
                        .expect("Failed to deserialize ForesterEpochPda");
                forester_epoch_pdas.push(forester_epoch_pda);
            }
            EpochPda::DISCRIMINATOR => {
                let epoch_pda = EpochPda::try_deserialize_unchecked(&mut account.data())
                    .expect("Failed to deserialize EpochPda");
                epoch_pdas.push(epoch_pda);
            }
            ProtocolConfigPda::DISCRIMINATOR => {
                let protocol_config_pda =
                    ProtocolConfigPda::try_deserialize_unchecked(&mut account.data())
                        .expect("Failed to deserialize ProtocolConfigPda");
                protocol_config_pdas.push(protocol_config_pda);
            }
            _ => (),
        }
    }
    forester_epoch_pdas.sort_by(|a, b| a.epoch.cmp(&b.epoch));
    epoch_pdas.sort_by(|a, b| a.epoch.cmp(&b.epoch));
    let slot = client.get_slot().expect("Failed to fetch slot.");
    let current_active_epoch = protocol_config_pdas[0]
        .config
        .get_current_active_epoch(slot)
        .unwrap();
    let current_registration_epoch = protocol_config_pdas[0]
        .config
        .get_latest_register_epoch(slot)
        .unwrap();
    println!("Current active epoch: {:?}", current_active_epoch);

    println!(
        "Current registration epoch: {:?}",
        current_registration_epoch
    );

    println!(
        "Forester registered for latest epoch: {:?}",
        forester_epoch_pdas
            .iter()
            .any(|pda| pda.epoch == current_registration_epoch)
    );
    println!(
        "Forester registered for active epoch: {:?}",
        forester_epoch_pdas
            .iter()
            .any(|pda| pda.epoch == current_active_epoch)
    );
    println!(
        "current active epoch progress {:?} / {}",
        protocol_config_pdas[0]
            .config
            .get_current_active_epoch_progress(slot),
        protocol_config_pdas[0].config.active_phase_length
    );
    println!(
        "current active epoch progress {:?}%",
        protocol_config_pdas[0]
            .config
            .get_current_active_epoch_progress(slot) as f64
            / protocol_config_pdas[0].config.active_phase_length as f64
            * 100f64
    );
    println!("Hours until next epoch : {:?} hours", {
        // slotduration is 460ms and 1000ms is 1 second and 3600 seconds is 1 hour
        protocol_config_pdas[0]
            .config
            .active_phase_length
            .saturating_sub(
                protocol_config_pdas[0]
                    .config
                    .get_current_active_epoch_progress(slot),
            )
            * 460
            / 1000
            / 3600
    });
    let slots_until_next_registration = protocol_config_pdas[0]
        .config
        .registration_phase_length
        .saturating_sub(
            protocol_config_pdas[0]
                .config
                .get_current_active_epoch_progress(slot),
        );
    println!(
        "Slots until next registration : {:?}",
        slots_until_next_registration
    );
    println!(
        "Hours until next registration : {:?} hours",
        // slotduration is 460ms and 1000ms is 1 second and 3600 seconds is 1 hour
        slots_until_next_registration * 460 / 1000 / 3600
    );
    if opts.full {
        for epoch in &epoch_pdas {
            println!("Epoch: {:?}", epoch.epoch);
            let registered_foresters_in_epoch = forester_epoch_pdas
                .iter()
                .filter(|pda| pda.epoch == epoch.epoch);
            for forester in registered_foresters_in_epoch {
                println!("Forester authority: {:?}", forester.authority);
            }
        }
    }
    if opts.protocol_config {
        println!("protocol config: {:?}", protocol_config_pdas[0]);
    }
    if opts.queue {
        let account_compression_accounts = client
            .get_program_accounts(&account_compression::ID)
            .expect("Failed to fetch accounts for account compression program.");
        for (pubkey, mut account) in account_compression_accounts {
            match account.data()[0..8].try_into().unwrap() {
                QueueAccount::DISCRIMINATOR => {
                    unsafe {
                        let queue = HashSet::from_bytes_copy(
                            &mut account.data[8 + std::mem::size_of::<QueueAccount>()..],
                        )
                        .unwrap();

                        println!("Queue account: {:?}", pubkey);
                        let mut num_of_marked_items = 0;
                        for i in 0..queue.get_capacity() {
                            if queue.get_unmarked_bucket(i).is_some() {
                                num_of_marked_items += 1;
                            }
                        }
                        println!(
                            "queue num of unmarked items: {:?} / {}",
                            num_of_marked_items,
                            queue.get_capacity() / 2 // div by 2 because only half of the hash set can be used before tx start to fail
                        );
                    }
                }
                StateMerkleTreeAccount::DISCRIMINATOR => {
                    println!("State Merkle tree: {:?}", pubkey);
                    let merkle_tree = ConcurrentMerkleTreeCopy::<Poseidon, 26>::from_bytes_copy(
                        &account.data[8 + std::mem::size_of::<StateMerkleTreeAccount>()..],
                    )
                    .unwrap();
                    println!(
                        "State Merkle tree next index {:?}",
                        merkle_tree.next_index()
                    );
                }
                AddressMerkleTreeAccount::DISCRIMINATOR => {
                    println!("Address Merkle tree: {:?}", pubkey);
                    let merkle_tree = ConcurrentMerkleTreeCopy::<Poseidon, 26>::from_bytes_copy(
                        &account.data[8 + std::mem::size_of::<AddressMerkleTreeAccount>()..],
                    )
                    .unwrap();
                    println!(
                        "Address Merkle tree next index {:?}",
                        merkle_tree.next_index()
                    );
                }
                _ => (),
            }
        }
    }

    Ok(())
}
