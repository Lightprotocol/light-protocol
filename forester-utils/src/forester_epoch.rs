use anchor_lang::{
    prelude::borsh, solana_program::pubkey::Pubkey, AnchorDeserialize, AnchorSerialize,
};
use light_client::rpc::{Rpc, RpcError};
use light_compressed_account::TreeType;
use light_registry::{
    protocol_config::state::{EpochState, ProtocolConfig},
    sdk::{create_register_forester_epoch_pda_instruction, create_report_work_instruction},
    utils::{get_epoch_pda_address, get_forester_epoch_pda_from_authority},
    EpochPda, ForesterEpochPda,
};
use solana_sdk::signature::{Keypair, Signature, Signer};

use crate::error::ForesterUtilsError;

// What does the forester need to know?
// What are my public keys (current epoch account, last epoch account, known Merkle trees)
// 1. The current epoch
// 2. When does the next registration start
// 3. When is my turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForesterSlot {
    pub slot: u64,
    pub start_solana_slot: u64,
    pub end_solana_slot: u64,
    pub forester_index: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Forester {
    pub registration: Epoch,
    pub active: Epoch,
    pub report_work: Epoch,
}

impl Forester {
    pub fn switch_to_report_work(&mut self) {
        self.report_work = self.active.clone();
        self.active = self.registration.clone();
    }

    pub async fn report_work(
        &mut self,
        rpc: &mut impl Rpc,
        forester_keypair: &Keypair,
        derivation: &Pubkey,
    ) -> Result<Signature, RpcError> {
        let ix = create_report_work_instruction(
            &forester_keypair.pubkey(),
            derivation,
            self.report_work.epoch,
        );
        rpc.create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[forester_keypair])
            .await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeAccounts {
    pub merkle_tree: Pubkey,
    pub queue: Pubkey,
    // TODO: evaluate whether we need
    pub is_rolledover: bool,
    pub tree_type: TreeType,
}

impl TreeAccounts {
    pub fn new(
        merkle_tree: Pubkey,
        queue: Pubkey,
        tree_type: TreeType,
        is_rolledover: bool,
    ) -> Self {
        Self {
            merkle_tree,
            queue,
            tree_type,
            is_rolledover,
        }
    }
}

pub fn get_schedule_for_queue(
    mut start_solana_slot: u64,
    queue_pubkey: &Pubkey,
    protocol_config: &ProtocolConfig,
    total_epoch_weight: u64,
    epoch: u64,
    current_phase_start_slot: u64,
) -> Result<Vec<Option<ForesterSlot>>, ForesterUtilsError> {
    let mut vec = Vec::new();

    let current_light_slot = if start_solana_slot >= current_phase_start_slot {
        (start_solana_slot - current_phase_start_slot) / protocol_config.slot_length
    } else {
        return Err(ForesterUtilsError::InvalidSlotNumber);
    };

    let start_slot = current_light_slot;
    start_solana_slot =
        current_phase_start_slot + (current_light_slot * protocol_config.slot_length);
    let end_slot = protocol_config.active_phase_length / protocol_config.slot_length;

    for light_slot in start_slot..end_slot {
        let forester_index = ForesterEpochPda::get_eligible_forester_index(
            light_slot,
            queue_pubkey,
            total_epoch_weight,
            epoch,
        )
        .unwrap();
        vec.push(Some(ForesterSlot {
            slot: light_slot,
            start_solana_slot,
            end_solana_slot: start_solana_slot + protocol_config.slot_length,
            forester_index,
        }));
        start_solana_slot += protocol_config.slot_length;
    }
    Ok(vec)
}

pub fn get_schedule_for_forester_in_queue(
    start_solana_slot: u64,
    queue_pubkey: &Pubkey,
    total_epoch_weight: u64,
    forester_epoch_pda: &ForesterEpochPda,
) -> Result<Vec<Option<ForesterSlot>>, ForesterUtilsError> {
    let mut slots = get_schedule_for_queue(
        start_solana_slot,
        queue_pubkey,
        &forester_epoch_pda.protocol_config,
        total_epoch_weight,
        forester_epoch_pda.epoch,
        forester_epoch_pda.epoch_active_phase_start_slot,
    )?;
    slots.iter_mut().for_each(|slot_option| {
        if let Some(slot) = slot_option {
            if !forester_epoch_pda.is_eligible(slot.forester_index) {
                *slot_option = None;
            }
        }
    });
    Ok(slots)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeForesterSchedule {
    pub tree_accounts: TreeAccounts,
    /// Vec with the slots that the forester is eligible to perform work.
    /// Non-eligible slots are None.
    pub slots: Vec<Option<ForesterSlot>>,
}

impl TreeForesterSchedule {
    pub fn new(tree_accounts: TreeAccounts) -> Self {
        Self {
            tree_accounts,
            slots: Vec::new(),
        }
    }

    pub fn new_with_schedule(
        tree_accounts: &TreeAccounts,
        solana_slot: u64,
        forester_epoch_pda: &ForesterEpochPda,
        epoch_pda: &EpochPda,
    ) -> Result<Self, ForesterUtilsError> {
        let mut _self = Self {
            tree_accounts: *tree_accounts,
            slots: Vec::new(),
        };
        _self.slots = get_schedule_for_forester_in_queue(
            solana_slot,
            &_self.tree_accounts.queue,
            epoch_pda.registered_weight,
            forester_epoch_pda,
        )?;
        Ok(_self)
    }

    pub fn is_eligible(&self, forester_slot: u64) -> bool {
        self.slots[forester_slot as usize].is_some()
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, Default, PartialEq, Eq)]
pub struct EpochPhases {
    pub registration: Phase,
    pub active: Phase,
    pub report_work: Phase,
    pub post: Phase,
}

impl EpochPhases {
    pub fn get_current_phase(&self, current_slot: u64) -> Phase {
        if current_slot >= self.registration.start && current_slot <= self.registration.end {
            self.registration.clone()
        } else if current_slot >= self.active.start && current_slot <= self.active.end {
            self.active.clone()
        } else if current_slot >= self.report_work.start && current_slot <= self.report_work.end {
            self.report_work.clone()
        } else {
            self.post.clone()
        }
    }
    pub fn get_current_epoch_state(&self, current_slot: u64) -> EpochState {
        if current_slot >= self.registration.start && current_slot <= self.registration.end {
            EpochState::Registration
        } else if current_slot >= self.active.start && current_slot <= self.active.end {
            EpochState::Active
        } else if current_slot >= self.report_work.start && current_slot <= self.report_work.end {
            EpochState::ReportWork
        } else {
            EpochState::Post
        }
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, Default, PartialEq, Eq)]
pub struct Phase {
    pub start: u64,
    pub end: u64,
}

impl Phase {
    pub fn length(&self) -> u64 {
        self.end - self.start
    }
}

pub fn get_epoch_phases(protocol_config: &ProtocolConfig, epoch: u64) -> EpochPhases {
    let epoch_start_slot = protocol_config
        .genesis_slot
        .saturating_add(epoch.saturating_mul(protocol_config.active_phase_length));

    let registration_start = epoch_start_slot;
    let registration_end = registration_start
        .saturating_add(protocol_config.registration_phase_length)
        .saturating_sub(1);

    let active_start = registration_end.saturating_add(1);
    let active_end = active_start
        .saturating_add(protocol_config.active_phase_length)
        .saturating_sub(1);

    let report_work_start = active_end.saturating_add(1);
    let report_work_end = report_work_start
        .saturating_add(protocol_config.report_work_phase_length)
        .saturating_sub(1);

    let post_start = report_work_end.saturating_add(1);
    let post_end = u64::MAX;

    EpochPhases {
        registration: Phase {
            start: registration_start,
            end: registration_end,
        },
        active: Phase {
            start: active_start,
            end: active_end,
        },
        report_work: Phase {
            start: report_work_start,
            end: report_work_end,
        },
        post: Phase {
            start: post_start,
            end: post_end,
        },
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Epoch {
    pub epoch: u64,
    pub epoch_pda: Pubkey,
    pub forester_epoch_pda: Pubkey,
    pub phases: EpochPhases,
    pub state: EpochState,
    pub merkle_trees: Vec<TreeForesterSchedule>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, Default, PartialEq, Eq)]
pub struct EpochRegistration {
    pub epoch: u64,
    pub slots_until_registration_starts: u64,
    pub slots_until_registration_ends: u64,
}

impl Epoch {
    /// returns slots until next epoch and that epoch
    /// registration is open if
    pub async fn slots_until_next_epoch_registration<R: Rpc>(
        rpc: &mut R,
        protocol_config: &ProtocolConfig,
    ) -> Result<EpochRegistration, RpcError> {
        let current_solana_slot = rpc.get_slot().await?;

        let mut epoch = protocol_config
            .get_latest_register_epoch(current_solana_slot)
            .unwrap();
        let registration_start_slot =
            protocol_config.genesis_slot + epoch * protocol_config.active_phase_length;

        let registration_end_slot =
            registration_start_slot + protocol_config.registration_phase_length;
        if current_solana_slot > registration_end_slot {
            epoch += 1;
        }
        let next_registration_start_slot =
            protocol_config.genesis_slot + epoch * protocol_config.active_phase_length;
        let next_registration_end_slot =
            next_registration_start_slot + protocol_config.registration_phase_length;
        let slots_until_registration_ends =
            next_registration_end_slot.saturating_sub(current_solana_slot);
        let slots_until_registration_starts =
            next_registration_start_slot.saturating_sub(current_solana_slot);
        Ok(EpochRegistration {
            epoch,
            slots_until_registration_starts,
            slots_until_registration_ends,
        })
    }

    /// creates forester account and fetches epoch account
    pub async fn register<R: Rpc>(
        rpc: &mut R,
        protocol_config: &ProtocolConfig,
        authority: &Keypair,
        derivation: &Pubkey,
    ) -> Result<Option<Epoch>, RpcError> {
        let epoch_registration =
            Self::slots_until_next_epoch_registration(rpc, protocol_config).await?;
        if epoch_registration.slots_until_registration_starts > 0
            || epoch_registration.slots_until_registration_ends == 0
        {
            return Ok(None);
        }

        let instruction = create_register_forester_epoch_pda_instruction(
            &authority.pubkey(),
            derivation,
            epoch_registration.epoch,
        );
        let signature = rpc
            .create_and_send_transaction(&[instruction], &authority.pubkey(), &[authority])
            .await?;
        rpc.confirm_transaction(signature).await?;
        let epoch_pda_pubkey = get_epoch_pda_address(epoch_registration.epoch);
        let epoch_pda = rpc
            .get_anchor_account::<EpochPda>(&epoch_pda_pubkey)
            .await?
            .unwrap();
        let forester_epoch_pda_pubkey =
            get_forester_epoch_pda_from_authority(derivation, epoch_registration.epoch).0;

        let phases = get_epoch_phases(protocol_config, epoch_pda.epoch);
        Ok(Some(Self {
            // epoch: epoch_registration.epoch,
            epoch_pda: epoch_pda_pubkey,
            forester_epoch_pda: forester_epoch_pda_pubkey,
            merkle_trees: Vec::new(),
            epoch: epoch_pda.epoch,
            state: phases.get_current_epoch_state(rpc.get_slot().await?),
            phases,
        }))
    }
    // TODO: implement
    /// forester account and epoch account already exist
    /// -> fetch accounts and init
    pub fn fetch_registered() {}

    pub async fn fetch_account_and_add_trees_with_schedule<R: Rpc>(
        &mut self,
        rpc: &mut R,
        trees: &[TreeAccounts],
    ) -> Result<(), RpcError> {
        let current_solana_slot = rpc.get_slot().await?;

        if self.phases.active.end < current_solana_slot
            || self.phases.active.start > current_solana_slot
        {
            println!("current_solana_slot {:?}", current_solana_slot);
            println!("registration phase {:?}", self.phases.registration);
            println!("active phase {:?}", self.phases.active);
            // return Err(RpcError::EpochNotActive);
            panic!("TODO: throw epoch not active error");
        }
        let epoch_pda = rpc
            .get_anchor_account::<EpochPda>(&self.epoch_pda)
            .await?
            .unwrap();
        let mut forester_epoch_pda = rpc
            .get_anchor_account::<ForesterEpochPda>(&self.forester_epoch_pda)
            .await?
            .unwrap();
        // IF active phase has started and total_epoch_weight is not set, set it now to
        if forester_epoch_pda.total_epoch_weight.is_none() {
            forester_epoch_pda.total_epoch_weight = Some(epoch_pda.registered_weight);
        }
        self.add_trees_with_schedule(&forester_epoch_pda, &epoch_pda, trees, current_solana_slot)
            .map_err(|e| {
                println!("Error adding trees with schedule: {:?}", e);
                RpcError::AssertRpcError("Error adding trees with schedule".to_string())
            })?;
        Ok(())
    }
    /// Internal function to init Epoch struct with registered account
    /// 1. calculate epoch phases
    /// 2. set current epoch state
    /// 3. derive tree schedule for all input trees
    pub fn add_trees_with_schedule(
        &mut self,
        forester_epoch_pda: &ForesterEpochPda,
        epoch_pda: &EpochPda,
        trees: &[TreeAccounts],
        current_solana_slot: u64,
    ) -> Result<(), ForesterUtilsError> {
        // TODO: add epoch state to sync schedule
        for tree in trees {
            let tree_schedule = TreeForesterSchedule::new_with_schedule(
                tree,
                current_solana_slot,
                forester_epoch_pda,
                epoch_pda,
            )?;
            self.merkle_trees.push(tree_schedule);
        }
        Ok(())
    }

    pub fn update_state(&mut self, current_solana_slot: u64) -> EpochState {
        let current_state = self.phases.get_current_epoch_state(current_solana_slot);
        if current_state != self.state {
            self.state = current_state.clone();
        }
        current_state
    }

    /// execute active phase test:
    /// (multi thread)
    /// - iterate over all trees, check whether eligible and empty queues
    ///
    /// forester:
    /// - start a new thread per tree
    /// - this thread will sleep when it is not eligible and wake up with
    ///   some buffer time prior to the start of the slot
    /// - threads shut down when the active phase ends
    pub fn execute_active_phase() {}

    /// report work phase:
    /// (single thread)
    /// - free Merkle tree memory
    /// - execute report work tx (single thread)
    pub fn execute_report_work_phase() {}
    /// post phase:
    /// (single thread)
    /// - claim rewards
    /// - close forester epoch account
    pub fn execute_post_phase() {}
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_epoch_phases() {
        let config = ProtocolConfig {
            genesis_slot: 200,
            min_weight: 0,
            slot_length: 10,
            registration_phase_length: 100,
            active_phase_length: 1000,
            report_work_phase_length: 100,
            network_fee: 5000,
            ..Default::default()
        };

        let epoch = 1;
        let phases = get_epoch_phases(&config, epoch);

        assert_eq!(phases.registration.start, 1200);
        assert_eq!(phases.registration.end, 1299);

        assert_eq!(phases.active.start, 1300);
        assert_eq!(phases.active.end, 2299);

        assert_eq!(phases.report_work.start, 2300);
        assert_eq!(phases.report_work.end, 2399);

        assert_eq!(phases.post.start, 2400);
        assert_eq!(phases.post.end, u64::MAX);
    }

    #[test]
    fn test_get_schedule_for_queue() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 0,
            min_weight: 100,
            slot_length: 10,
            registration_phase_length: 100,
            active_phase_length: 1000,
            report_work_phase_length: 100,
            network_fee: 5000,
            ..Default::default()
        };

        let total_epoch_weight = 500;
        let queue_pubkey = Pubkey::new_unique();
        let start_solana_slot = 0;
        let epoch = 0;
        let current_phase_start_slot = 0;

        let schedule = get_schedule_for_queue(
            start_solana_slot,
            &queue_pubkey,
            &protocol_config,
            total_epoch_weight,
            epoch,
            current_phase_start_slot,
        )
        .unwrap();

        // Expected number of light slots in the active phase
        let expected_light_slots =
            (protocol_config.active_phase_length / protocol_config.slot_length) as usize;
        assert_eq!(schedule.len(), expected_light_slots); // Should generate 100 slots

        assert_eq!(
            schedule.len(),
            (protocol_config.active_phase_length / protocol_config.slot_length) as usize
        );

        for (i, slot_option) in schedule.iter().enumerate() {
            let slot = slot_option.as_ref().unwrap();
            assert_eq!(slot.slot, i as u64);
            assert_eq!(
                slot.start_solana_slot,
                start_solana_slot + (i as u64 * protocol_config.slot_length)
            );
            assert_eq!(
                slot.end_solana_slot,
                slot.start_solana_slot + protocol_config.slot_length
            );
            assert!(slot.forester_index < total_epoch_weight);
        }
    }

    #[test]
    fn test_get_schedule_for_queue_offset_phase_start() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 1000, // Genesis starts later
            min_weight: 100,
            slot_length: 10,
            registration_phase_length: 100,
            active_phase_length: 1000, // 100 light slots
            report_work_phase_length: 100,
            network_fee: 5000,
            ..Default::default()
        };

        let total_epoch_weight = 500;
        let queue_pubkey = Pubkey::new_unique();
        let epoch = 0;

        // Calculate actual start of the active phase for epoch 0
        // Registration: 1000 to 1099
        // Active: 1100 to 2099
        let current_phase_start_slot = 1100;

        // Start calculating right from the beginning of this active phase
        let start_solana_slot = current_phase_start_slot;

        let schedule = get_schedule_for_queue(
            start_solana_slot,
            &queue_pubkey,
            &protocol_config,
            total_epoch_weight,
            epoch,
            current_phase_start_slot, // Pass the calculated start slot
        )
        .unwrap();

        let expected_light_slots =
            (protocol_config.active_phase_length / protocol_config.slot_length) as usize;
        assert_eq!(schedule.len(), expected_light_slots); // Still 100 light slots expected

        // Check the first slot details
        let first_slot = schedule[0].as_ref().unwrap();
        assert_eq!(first_slot.slot, 0); // First light slot index is 0
                                        // Its Solana start slot should be the phase start slot
        assert_eq!(first_slot.start_solana_slot, current_phase_start_slot);
        assert_eq!(
            first_slot.end_solana_slot,
            current_phase_start_slot + protocol_config.slot_length
        );

        // Check the second slot details
        let second_slot = schedule[1].as_ref().unwrap();
        assert_eq!(second_slot.slot, 1); // Second light slot index is 1
                                         // Its Solana start slot should be offset by one slot_length
        assert_eq!(
            second_slot.start_solana_slot,
            current_phase_start_slot + protocol_config.slot_length
        );
        assert_eq!(
            second_slot.end_solana_slot,
            current_phase_start_slot + 2 * protocol_config.slot_length
        );
    }

    // NEW TEST: Case where current_light_slot > 0
    #[test]
    fn test_get_schedule_for_queue_mid_phase_start() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 0,
            min_weight: 100,
            slot_length: 10,
            registration_phase_length: 100, // Reg: 0-99
            active_phase_length: 1000,      // Active: 100-1099 (100 light slots)
            report_work_phase_length: 100,
            network_fee: 5000,
            ..Default::default()
        };

        let total_epoch_weight = 500;
        let queue_pubkey = Pubkey::new_unique();
        let epoch = 0;
        let current_phase_start_slot = 100; // Active phase starts at slot 100

        // Start calculating from Solana slot 155, which is within the active phase
        let start_solana_slot = 155;

        // Calculation:
        // current_light_slot = floor((155 - 100) / 10) = floor(55 / 10) = 5
        // Effective start_solana_slot for loop = 100 + (5 * 10) = 150
        // End light slot = 1000 / 10 = 100
        // Loop runs from light_slot 5 to 99 (inclusive). Length = 100 - 5 = 95

        let schedule = get_schedule_for_queue(
            start_solana_slot,
            &queue_pubkey,
            &protocol_config,
            total_epoch_weight,
            epoch,
            current_phase_start_slot,
        )
        .unwrap();

        let expected_light_slots_total =
            protocol_config.active_phase_length / protocol_config.slot_length; // 100
        let expected_start_light_slot = 5;
        let expected_schedule_len =
            (expected_light_slots_total - expected_start_light_slot) as usize; // 100 - 5 = 95

        assert_eq!(schedule.len(), expected_schedule_len); // Should generate 95 slots

        // Check the first slot in the *returned* schedule
        let first_returned_slot = schedule[0].as_ref().unwrap();
        assert_eq!(first_returned_slot.slot, expected_start_light_slot); // Light slot index starts at 5
                                                                         // Its Solana start slot should align to the beginning of light slot 5
        let expected_first_solana_start =
            current_phase_start_slot + expected_start_light_slot * protocol_config.slot_length; // 100 + 5 * 10 = 150
        assert_eq!(
            first_returned_slot.start_solana_slot,
            expected_first_solana_start
        );
        assert_eq!(
            first_returned_slot.end_solana_slot,
            expected_first_solana_start + protocol_config.slot_length // 150 + 10 = 160
        );

        // Check the second slot in the *returned* schedule
        let second_returned_slot = schedule[1].as_ref().unwrap();
        assert_eq!(second_returned_slot.slot, expected_start_light_slot + 1); // Light slot index 6
                                                                              // Its Solana start slot should be 160
        assert_eq!(
            second_returned_slot.start_solana_slot,
            expected_first_solana_start + protocol_config.slot_length
        );
        assert_eq!(
            second_returned_slot.end_solana_slot,
            expected_first_solana_start + 2 * protocol_config.slot_length // 170
        );
    }
}
