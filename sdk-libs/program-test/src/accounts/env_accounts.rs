use forester_utils::forester_epoch::Epoch;
use light_registry::account_compression_cpi::sdk::get_registered_program_pda;
use light_system_program;
use solana_sdk::{pubkey, pubkey::Pubkey, signature::Keypair};

use super::env_keypairs::{FORESTER_TEST_KEYPAIR, PAYER_KEYPAIR};

pub const NOOP_PROGRAM_ID: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

#[derive(Debug)]
pub struct EnvAccounts {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub governance_authority: Keypair,
    pub governance_authority_pda: Pubkey,
    pub group_pda: Pubkey,
    pub forester: Keypair,
    pub registered_program_pda: Pubkey,
    pub registered_registry_program_pda: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_queue_pubkey: Pubkey,
    pub cpi_context_account_pubkey: Pubkey,
    pub registered_forester_pda: Pubkey,
    pub forester_epoch: Option<Epoch>,
    pub batched_state_merkle_tree: Pubkey,
    pub batched_output_queue: Pubkey,
    pub batched_cpi_context: Pubkey,
    pub batch_address_merkle_tree: Pubkey,
}

impl EnvAccounts {
    pub fn get_local_test_validator_accounts() -> EnvAccounts {
        EnvAccounts {
            merkle_tree_pubkey: pubkey!("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT"),
            nullifier_queue_pubkey: pubkey!("nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148"),
            governance_authority: Keypair::from_bytes(&PAYER_KEYPAIR).unwrap(),
            governance_authority_pda: Pubkey::default(),
            group_pda: Pubkey::default(),
            forester: Keypair::from_bytes(&FORESTER_TEST_KEYPAIR).unwrap(),
            registered_program_pda: get_registered_program_pda(&light_system_program::ID),
            registered_registry_program_pda: get_registered_program_pda(&light_registry::ID),
            address_merkle_tree_pubkey: pubkey!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2"),
            address_merkle_tree_queue_pubkey: pubkey!(
                "aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F"
            ),
            cpi_context_account_pubkey: pubkey!("cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4"),
            registered_forester_pda: Pubkey::default(),
            forester_epoch: None, // Set to None or to an appropriate Epoch value if needed
            batched_state_merkle_tree: pubkey!("HLKs5NJ8FXkJg8BrzJt56adFYYuwg5etzDtBbQYTsixu"),
            batched_output_queue: pubkey!("6L7SzhYB3anwEQ9cphpJ1U7Scwj57bx2xueReg7R9cKU"),
            batched_cpi_context: pubkey!("7Hp52chxaew8bW1ApR4fck2bh6Y8qA1pu3qwH6N9zaLj"),
            batch_address_merkle_tree: pubkey!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK"),
        }
    }
}
