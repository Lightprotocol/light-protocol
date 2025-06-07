use light_client::rpc::Rpc;
use light_registry::{
    protocol_config::state::ProtocolConfigPda,
    utils::{get_epoch_pda_address, get_forester_pda, get_protocol_config_pda_address},
    EpochPda, ForesterEpochPda, ForesterPda,
};
use solana_sdk::pubkey::Pubkey;

pub async fn assert_finalized_epoch_registration<R: Rpc>(
    rpc: &mut R,
    forester_epoch_pda_pubkey: &Pubkey,
    epoch_pda_pubkey: &Pubkey,
) {
    let epoch_pda = rpc
        .get_anchor_account::<EpochPda>(epoch_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    let expected_total_epoch_weight = epoch_pda.registered_weight;
    let forester_epoch_pda = rpc
        .get_anchor_account::<ForesterEpochPda>(forester_epoch_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    assert!(forester_epoch_pda.total_epoch_weight.is_some());
    assert_eq!(
        forester_epoch_pda.total_epoch_weight.unwrap(),
        expected_total_epoch_weight
    );
}

pub async fn assert_epoch_pda<R: Rpc>(rpc: &mut R, epoch: u64, expected_registered_weight: u64) {
    let epoch_pda_pubkey = get_epoch_pda_address(epoch);
    let epoch_pda = rpc
        .get_anchor_account::<EpochPda>(&epoch_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    let protocol_config_pda_pubkey = get_protocol_config_pda_address().0;
    let protocol_config_pda = rpc
        .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(epoch_pda.registered_weight, expected_registered_weight);
    assert_eq!(epoch_pda.total_work, 0);
    assert_eq!(epoch_pda.protocol_config, protocol_config_pda.config);
    assert_eq!(epoch_pda.epoch, epoch);
}
/// Helper function to fetch the forester epoch and epoch account to assert diff
/// after transaction.
pub async fn fetch_epoch_and_forester_pdas<R: Rpc>(
    rpc: &mut R,
    forester_epoch_pda: &Pubkey,
    epoch_pda: &Pubkey,
) -> (ForesterEpochPda, EpochPda) {
    let forester_epoch_pda = rpc
        .get_anchor_account::<ForesterEpochPda>(forester_epoch_pda)
        .await
        .unwrap()
        .unwrap();
    println!("forester_epoch_pda: {:?}", forester_epoch_pda);
    let epoch_pda = rpc
        .get_anchor_account::<EpochPda>(epoch_pda)
        .await
        .unwrap()
        .unwrap();
    println!("epoch_pda: {:?}", epoch_pda);

    (forester_epoch_pda, epoch_pda)
}

/// Asserts:
/// 1. ForesterEpochPda has reported work
/// 2. EpochPda has updated total work by forester work counter
pub async fn assert_report_work<R: Rpc>(
    rpc: &mut R,
    forester_epoch_pda_pubkey: &Pubkey,
    epoch_pda_pubkey: &Pubkey,
    mut pre_forester_epoch_pda: ForesterEpochPda,
    mut pre_epoch_pda: EpochPda,
) {
    let forester_epoch_pda = rpc
        .get_anchor_account::<ForesterEpochPda>(forester_epoch_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    pre_forester_epoch_pda.has_reported_work = true;
    assert_eq!(forester_epoch_pda, pre_forester_epoch_pda);
    let epoch_pda = rpc
        .get_anchor_account::<EpochPda>(epoch_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    pre_epoch_pda.total_work += forester_epoch_pda.work_counter;
    assert_eq!(epoch_pda, pre_epoch_pda);
}

/// Asserts the correct creation of a ForesterEpochPda.
pub async fn assert_registered_forester_pda<R: Rpc>(
    rpc: &mut R,
    forester_epoch_pda_pubkey: &Pubkey,
    forester_derivation_pubkey: &Pubkey,
    epoch: u64,
) {
    let (forester_pda_pubkey, _) = get_forester_pda(forester_derivation_pubkey);

    let epoch_pda_pubkey = get_epoch_pda_address(epoch);
    let epoch_pda = rpc
        .get_anchor_account::<EpochPda>(&epoch_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    let forester_pda = rpc
        .get_anchor_account::<ForesterPda>(&forester_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    let epoch_active_phase_start_slot = epoch_pda.protocol_config.genesis_slot
        + epoch_pda.protocol_config.registration_phase_length
        + epoch_pda.epoch * epoch_pda.protocol_config.active_phase_length;
    let expected_forester_epoch_pda = ForesterEpochPda {
        authority: forester_pda.authority,
        config: forester_pda.config,
        epoch: epoch_pda.epoch,
        weight: forester_pda.active_weight,
        work_counter: 0,
        has_reported_work: false,
        forester_index: epoch_pda.registered_weight - forester_pda.active_weight,
        total_epoch_weight: None,
        epoch_active_phase_start_slot,
        protocol_config: epoch_pda.protocol_config,
        finalize_counter: 0,
    };
    let forester_epoch_pda = rpc
        .get_anchor_account::<ForesterEpochPda>(forester_epoch_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(forester_epoch_pda, expected_forester_epoch_pda);
}
