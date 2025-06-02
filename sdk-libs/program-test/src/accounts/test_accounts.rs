use forester_utils::forester_epoch::Epoch;
use light_client::indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts};
use light_registry::{
    account_compression_cpi::sdk::get_registered_program_pda,
    sdk::create_register_program_instruction,
    utils::{get_forester_pda, get_protocol_config_pda_address},
};
use solana_sdk::{
    pubkey,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

use super::{initialize::*, test_keypairs::*};

pub const NOOP_PROGRAM_ID: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

#[derive(Debug)]
pub struct ProtocolAccounts {
    pub governance_authority: Keypair,
    pub governance_authority_pda: Pubkey,
    pub group_pda: Pubkey,
    pub forester: Keypair,
    pub registered_program_pda: Pubkey,
    pub registered_registry_program_pda: Pubkey,
    pub registered_forester_pda: Pubkey,
    pub forester_epoch: Option<Epoch>,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct StateMerkleTreeAccountsV2 {
    pub merkle_tree: Pubkey,
    pub output_queue: Pubkey,
    pub cpi_context: Pubkey,
}

#[derive(Debug)]
pub struct TestAccounts {
    pub protocol: ProtocolAccounts,
    pub v1_state_trees: Vec<StateMerkleTreeAccounts>,
    pub v1_address_trees: Vec<AddressMerkleTreeAccounts>,
    pub v2_state_trees: Vec<StateMerkleTreeAccountsV2>,
    pub v2_address_trees: Vec<Pubkey>,
}

impl TestAccounts {
    pub fn get_local_test_validator_accounts() -> TestAccounts {
        TestAccounts {
            protocol: ProtocolAccounts {
                governance_authority: Keypair::from_bytes(&PAYER_KEYPAIR).unwrap(),
                governance_authority_pda: Pubkey::default(),
                group_pda: Pubkey::default(),
                forester: Keypair::from_bytes(&FORESTER_TEST_KEYPAIR).unwrap(),
                registered_program_pda: get_registered_program_pda(
                    &light_sdk::constants::PROGRAM_ID_LIGHT_SYSTEM,
                ),
                registered_registry_program_pda: get_registered_program_pda(&light_registry::ID),
                registered_forester_pda: Pubkey::default(),
                forester_epoch: None, // Set to None or to an appropriate Epoch value if needed
            },
            v1_state_trees: vec![StateMerkleTreeAccounts {
                merkle_tree: pubkey!("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT"),
                nullifier_queue: pubkey!("nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148"),
                cpi_context: pubkey!("cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4"),
            }],

            v1_address_trees: vec![AddressMerkleTreeAccounts {
                merkle_tree: pubkey!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2"),
                queue: pubkey!("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F"),
            }],

            v2_address_trees: vec![pubkey!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK")],
            v2_state_trees: vec![StateMerkleTreeAccountsV2 {
                merkle_tree: pubkey!("HLKs5NJ8FXkJg8BrzJt56adFYYuwg5etzDtBbQYTsixu"),
                output_queue: pubkey!("6L7SzhYB3anwEQ9cphpJ1U7Scwj57bx2xueReg7R9cKU"),
                cpi_context: pubkey!("7Hp52chxaew8bW1ApR4fck2bh6Y8qA1pu3qwH6N9zaLj"),
            }],
        }
    }

    pub fn get_program_test_test_accounts() -> TestAccounts {
        let merkle_tree_keypair = Keypair::from_bytes(&MERKLE_TREE_TEST_KEYPAIR).unwrap();
        let nullifier_queue_keypair = Keypair::from_bytes(&NULLIFIER_QUEUE_TEST_KEYPAIR).unwrap();
        let group_seed_keypair = Keypair::from_bytes(&GROUP_PDA_SEED_TEST_KEYPAIR).unwrap();
        let group_pda = get_group_pda(group_seed_keypair.pubkey());

        let payer = Keypair::from_bytes(&PAYER_KEYPAIR).unwrap();
        let protocol_config_pda = get_protocol_config_pda_address();
        let (_, registered_program_pda) = create_register_program_instruction(
            payer.pubkey(),
            protocol_config_pda,
            group_pda,
            light_sdk::constants::PROGRAM_ID_LIGHT_SYSTEM,
        );

        let address_merkle_tree_keypair =
            Keypair::from_bytes(&ADDRESS_MERKLE_TREE_TEST_KEYPAIR).unwrap();

        let address_merkle_tree_queue_keypair =
            Keypair::from_bytes(&ADDRESS_MERKLE_TREE_QUEUE_TEST_KEYPAIR).unwrap();

        let cpi_context_keypair = Keypair::from_bytes(&SIGNATURE_CPI_TEST_KEYPAIR).unwrap();
        let registered_registry_program_pda = get_registered_program_pda(&light_registry::ID);
        let forester = Keypair::from_bytes(&FORESTER_TEST_KEYPAIR).unwrap();

        let forester_pubkey = forester.pubkey();
        TestAccounts {
            protocol: ProtocolAccounts {
                governance_authority: payer,
                governance_authority_pda: protocol_config_pda.0,
                group_pda,
                forester,
                registered_program_pda,
                registered_registry_program_pda,
                registered_forester_pda: get_forester_pda(&forester_pubkey).0,
                forester_epoch: None,
            },
            v1_state_trees: vec![StateMerkleTreeAccounts {
                merkle_tree: merkle_tree_keypair.pubkey(),
                nullifier_queue: nullifier_queue_keypair.pubkey(),
                cpi_context: cpi_context_keypair.pubkey(),
            }],
            v1_address_trees: vec![AddressMerkleTreeAccounts {
                merkle_tree: address_merkle_tree_keypair.pubkey(),
                queue: address_merkle_tree_queue_keypair.pubkey(),
            }],
            v2_state_trees: vec![StateMerkleTreeAccountsV2 {
                merkle_tree: Keypair::from_bytes(&BATCHED_STATE_MERKLE_TREE_TEST_KEYPAIR)
                    .unwrap()
                    .pubkey(),
                output_queue: Keypair::from_bytes(&BATCHED_OUTPUT_QUEUE_TEST_KEYPAIR)
                    .unwrap()
                    .pubkey(),
                cpi_context: Keypair::from_bytes(&BATCHED_CPI_CONTEXT_TEST_KEYPAIR)
                    .unwrap()
                    .pubkey(),
            }],
            v2_address_trees: vec![
                Keypair::from_bytes(&BATCHED_ADDRESS_MERKLE_TREE_TEST_KEYPAIR)
                    .unwrap()
                    .pubkey(),
            ],
        }
    }
}

impl Clone for TestAccounts {
    fn clone(&self) -> Self {
        TestAccounts {
            protocol: ProtocolAccounts {
                governance_authority: Keypair::from_bytes(
                    &self.protocol.governance_authority.to_bytes(),
                )
                .unwrap(),
                governance_authority_pda: self.protocol.governance_authority_pda,
                group_pda: self.protocol.group_pda,
                forester: Keypair::from_bytes(&self.protocol.forester.to_bytes()).unwrap(),
                registered_program_pda: self.protocol.registered_program_pda,
                registered_registry_program_pda: self.protocol.registered_registry_program_pda,
                registered_forester_pda: self.protocol.registered_forester_pda,
                forester_epoch: self.protocol.forester_epoch.clone(),
            },
            v1_state_trees: self.v1_state_trees.clone(),
            v1_address_trees: self.v1_address_trees.clone(),
            v2_state_trees: self.v2_state_trees.clone(),
            v2_address_trees: self.v2_address_trees.clone(),
        }
    }
}
