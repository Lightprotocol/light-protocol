use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
};
use light_prover_client::prover::ProverConfig;
use light_registry::protocol_config::state::ProtocolConfig;
use solana_sdk::pubkey::Pubkey;

use crate::logging::EnhancedLoggingConfig;

/// Configuration for Light Program Test
#[derive(Debug, Clone)]
pub struct ProgramTestConfig {
    pub additional_programs: Option<Vec<(&'static str, Pubkey)>>,
    pub protocol_config: ProtocolConfig,
    pub with_prover: bool,
    pub prover_config: Option<ProverConfig>,
    pub skip_register_programs: bool,
    pub skip_v1_trees: bool,
    pub skip_second_v1_tree: bool,
    pub v1_state_tree_config: StateMerkleTreeConfig,
    pub v1_nullifier_queue_config: NullifierQueueConfig,
    pub v1_address_tree_config: AddressMerkleTreeConfig,
    pub v1_address_queue_config: AddressQueueConfig,
    pub v2_state_tree_config: Option<InitStateTreeAccountsInstructionData>,
    pub v2_address_tree_config: Option<InitAddressTreeAccountsInstructionData>,
    pub skip_protocol_init: bool,
    /// Log failed transactions
    pub log_failed_tx: bool,
    /// Disable all logging
    pub no_logs: bool,
    /// Skip startup logs
    pub skip_startup_logs: bool,
    /// Log Light Protocol events (BatchPublicTransactionEvent, etc.)
    pub log_light_protocol_events: bool,
    /// Enhanced transaction logging configuration
    pub enhanced_logging: EnhancedLoggingConfig,
    /// Register a forester for epoch 0 during setup
    pub with_forester: bool,
}

impl ProgramTestConfig {
    pub fn new(
        with_prover: bool,
        additional_programs: Option<Vec<(&'static str, Pubkey)>>,
    ) -> Self {
        Self {
            additional_programs,
            with_prover,
            ..Default::default()
        }
    }

    #[cfg(feature = "v2")]
    pub fn new_v2(
        with_prover: bool,
        additional_programs: Option<Vec<(&'static str, Pubkey)>>,
    ) -> Self {
        let mut res = Self::default_with_batched_trees(with_prover);
        res.additional_programs = additional_programs;

        res
    }

    #[cfg(feature = "v2")]
    pub fn default_with_batched_trees(with_prover: bool) -> Self {
        Self {
            additional_programs: None,
            prover_config: Some(ProverConfig::default()),
            with_prover,
            v2_state_tree_config: Some(InitStateTreeAccountsInstructionData::test_default()),
            v2_address_tree_config: Some(InitAddressTreeAccountsInstructionData::test_default()),
            ..Default::default()
        }
    }

    #[cfg(feature = "devenv")]
    pub fn default_test_forester(with_prover: bool) -> Self {
        Self {
            additional_programs: None,
            with_prover,
            v2_state_tree_config: Some(InitStateTreeAccountsInstructionData::test_default()),
            v2_address_tree_config: Some(InitAddressTreeAccountsInstructionData::test_default()),
            prover_config: Some(ProverConfig::default()),
            ..Default::default()
        }
    }

    /// Enable Light Protocol event logging
    pub fn with_light_protocol_events(mut self) -> Self {
        self.log_light_protocol_events = true;
        self
    }

    /// Disable Light Protocol event logging
    pub fn without_light_protocol_events(mut self) -> Self {
        self.log_light_protocol_events = false;
        self
    }
}

impl Default for ProgramTestConfig {
    fn default() -> Self {
        Self {
            additional_programs: None,
            protocol_config: ProtocolConfig {
                // Init with an active epoch which doesn't end
                active_phase_length: 1_000_000_000,
                slot_length: 1_000_000_000 - 1,
                genesis_slot: 0,
                registration_phase_length: 2,
                ..Default::default()
            },
            with_prover: true,
            prover_config: None,
            skip_second_v1_tree: false,
            skip_register_programs: false,
            v1_state_tree_config: StateMerkleTreeConfig::default(),
            v1_address_tree_config: AddressMerkleTreeConfig::default(),
            v1_address_queue_config: AddressQueueConfig::default(),
            v1_nullifier_queue_config: NullifierQueueConfig::default(),
            v2_state_tree_config: None,
            v2_address_tree_config: None,
            skip_protocol_init: false,
            skip_v1_trees: false,
            log_failed_tx: true,
            no_logs: false,
            skip_startup_logs: true,
            log_light_protocol_events: false, // Disabled by default
            enhanced_logging: EnhancedLoggingConfig::from_env(),
            with_forester: true,
        }
    }
}
