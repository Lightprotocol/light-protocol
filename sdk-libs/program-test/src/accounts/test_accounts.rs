use light_client::indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts, TreeInfo};
use light_compressed_account::TreeType;
#[cfg(feature = "devenv")]
use light_registry::{
    account_compression_cpi::sdk::get_registered_program_pda,
    sdk::create_register_program_instruction,
    utils::{get_forester_pda, get_protocol_config_pda_address},
};
#[cfg(feature = "devenv")]
use solana_sdk::signature::Signer;
use solana_sdk::{pubkey, pubkey::Pubkey, signature::Keypair};

#[cfg(feature = "devenv")]
use super::initialize::*;
use super::test_keypairs::*;
#[cfg(feature = "devenv")]
use crate::compressible::FundingPoolConfig;

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
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct StateMerkleTreeAccountsV2 {
    pub merkle_tree: Pubkey,
    pub output_queue: Pubkey,
    pub cpi_context: Pubkey,
}

impl From<StateMerkleTreeAccountsV2> for TreeInfo {
    fn from(value: StateMerkleTreeAccountsV2) -> Self {
        TreeInfo {
            tree: value.merkle_tree,
            queue: value.output_queue,
            cpi_context: Some(value.cpi_context),
            tree_type: TreeType::StateV2,
            next_tree_info: None,
        }
    }
}

#[derive(Debug)]
pub struct TestAccounts {
    pub protocol: ProtocolAccounts,
    #[cfg(feature = "devenv")]
    pub funding_pool_config: FundingPoolConfig,
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
                registered_program_pda: pubkey!("35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh"),
                registered_registry_program_pda: pubkey!(
                    "DumMsyvkaGJG4QnQ1BhTgvoRMXsgGxfpKDUCr22Xqu4w"
                ),
                registered_forester_pda: Pubkey::default(),
            },
            v1_state_trees: vec![
                StateMerkleTreeAccounts {
                    merkle_tree: pubkey!("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT"),
                    nullifier_queue: pubkey!("nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148"),
                    cpi_context: pubkey!("cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4"),
                    tree_type: TreeType::StateV1,
                },
                StateMerkleTreeAccounts {
                    merkle_tree: pubkey!("smt2rJAFdyJJupwMKAqTNAJwvjhmiZ4JYGZmbVRw1Ho"),
                    nullifier_queue: pubkey!("nfq2hgS7NYemXsFaFUCe3EMXSDSfnZnAe27jC6aPP1X"),
                    cpi_context: pubkey!("cpi2cdhkH5roePvcudTgUL8ppEBfTay1desGh8G8QxK"),
                    tree_type: TreeType::StateV1,
                },
            ],

            v1_address_trees: vec![AddressMerkleTreeAccounts {
                merkle_tree: pubkey!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2"),
                queue: pubkey!("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F"),
            }],

            v2_address_trees: vec![pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx")],
            v2_state_trees: vec![
                StateMerkleTreeAccountsV2 {
                    merkle_tree: pubkey!("bmt1LryLZUMmF7ZtqESaw7wifBXLfXHQYoE4GAmrahU"),
                    output_queue: pubkey!("oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto"),
                    cpi_context: pubkey!("cpi15BoVPKgEPw5o8wc2T816GE7b378nMXnhH3Xbq4y"),
                },
                StateMerkleTreeAccountsV2 {
                    merkle_tree: pubkey!("bmt2UxoBxB9xWev4BkLvkGdapsz6sZGkzViPNph7VFi"),
                    output_queue: pubkey!("oq2UkeMsJLfXt2QHzim242SUi3nvjJs8Pn7Eac9H9vg"),
                    cpi_context: pubkey!("cpi2yGapXUR3As5SjnHBAVvmApNiLsbeZpF3euWnW6B"),
                },
                StateMerkleTreeAccountsV2 {
                    merkle_tree: pubkey!("bmt3ccLd4bqSVZVeCJnH1F6C8jNygAhaDfxDwePyyGb"),
                    output_queue: pubkey!("oq3AxjekBWgo64gpauB6QtuZNesuv19xrhaC1ZM1THQ"),
                    cpi_context: pubkey!("cpi3mbwMpSX8FAGMZVP85AwxqCaQMfEk9Em1v8QK9Rf"),
                },
                StateMerkleTreeAccountsV2 {
                    merkle_tree: pubkey!("bmt4d3p1a4YQgk9PeZv5s4DBUmbF5NxqYpk9HGjQsd8"),
                    output_queue: pubkey!("oq4ypwvVGzCUMoiKKHWh4S1SgZJ9vCvKpcz6RT6A8dq"),
                    cpi_context: pubkey!("cpi4yyPDc4bCgHAnsenunGA8Y77j3XEDyjgfyCKgcoc"),
                },
                StateMerkleTreeAccountsV2 {
                    merkle_tree: pubkey!("bmt5yU97jC88YXTuSukYHa8Z5Bi2ZDUtmzfkDTA2mG2"),
                    output_queue: pubkey!("oq5oh5ZR3yGomuQgFduNDzjtGvVWfDRGLuDVjv9a96P"),
                    cpi_context: pubkey!("cpi5ZTjdgYpZ1Xr7B1cMLLUE81oTtJbNNAyKary2nV6"),
                },
            ],
            #[cfg(feature = "devenv")]
            funding_pool_config: FundingPoolConfig::get_v1(),
        }
    }

    pub fn get_program_test_test_accounts() -> TestAccounts {
        #[cfg(feature = "devenv")]
        let (
            group_pda,
            protocol_config_pda,
            registered_program_pda,
            registered_registry_program_pda,
            registered_forester_pda,
        ) = {
            let group_seed_keypair = Keypair::from_bytes(&GROUP_PDA_SEED_TEST_KEYPAIR).unwrap();
            let group_pda = get_group_pda(group_seed_keypair.pubkey());
            let payer = Keypair::from_bytes(&PAYER_KEYPAIR).unwrap();
            let protocol_config_pda = get_protocol_config_pda_address();
            let (_, registered_program_pda) = create_register_program_instruction(
                payer.pubkey(),
                protocol_config_pda,
                group_pda,
                Pubkey::from(light_sdk::constants::LIGHT_SYSTEM_PROGRAM_ID),
            );
            let registered_registry_program_pda =
                get_registered_program_pda(&pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX"));
            let forester = Keypair::from_bytes(&FORESTER_TEST_KEYPAIR).unwrap();
            let registered_forester_pda = get_forester_pda(&forester.pubkey()).0;
            (
                group_pda,
                protocol_config_pda.0,
                registered_program_pda,
                registered_registry_program_pda,
                registered_forester_pda,
            )
        };

        #[cfg(not(feature = "devenv"))]
        let (
            group_pda,
            protocol_config_pda,
            registered_program_pda,
            registered_registry_program_pda,
            registered_forester_pda,
        ) = {
            // Hardcoded PDAs for non-devenv mode (these match the devenv calculations)
            let group_pda = pubkey!("Fomh1YizJdDfqvMJhC42cLNdcJM8NMM2NfxgZVEh3rkC");
            let protocol_config_pda = pubkey!("CuEtcKkkbTn6qy2qxqDswq5U2ADsqoipYDAYfRvxPjcp");
            let registered_program_pda = pubkey!("35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh");
            let registered_registry_program_pda =
                pubkey!("DumMsyvkaGJG4QnQ1BhTgvoRMXsgGxfpKDUCr22Xqu4w");
            let registered_forester_pda = pubkey!("3FBt1BPQHCQkS8k3wrUXMfB6JBhtMhEqQXueHRw2ojZV");
            (
                group_pda,
                protocol_config_pda,
                registered_program_pda,
                registered_registry_program_pda,
                registered_forester_pda,
            )
        };

        let payer = Keypair::from_bytes(&PAYER_KEYPAIR).unwrap();
        let forester = Keypair::from_bytes(&FORESTER_TEST_KEYPAIR).unwrap();

        TestAccounts {
            protocol: ProtocolAccounts {
                governance_authority: payer,
                governance_authority_pda: protocol_config_pda,
                group_pda,
                forester,
                registered_program_pda,
                registered_registry_program_pda,
                registered_forester_pda,
            },
            v1_state_trees: vec![
                StateMerkleTreeAccounts {
                    merkle_tree: pubkey!("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT"),
                    nullifier_queue: pubkey!("nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148"),
                    cpi_context: pubkey!("cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4"),
                    tree_type: TreeType::StateV1,
                },
                StateMerkleTreeAccounts {
                    merkle_tree: pubkey!("smt2rJAFdyJJupwMKAqTNAJwvjhmiZ4JYGZmbVRw1Ho"),
                    nullifier_queue: pubkey!("nfq2hgS7NYemXsFaFUCe3EMXSDSfnZnAe27jC6aPP1X"),
                    cpi_context: pubkey!("cpi2cdhkH5roePvcudTgUL8ppEBfTay1desGh8G8QxK"),
                    tree_type: TreeType::StateV1,
                },
            ],
            v1_address_trees: vec![AddressMerkleTreeAccounts {
                merkle_tree: pubkey!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2"),
                queue: pubkey!("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F"),
            }],
            v2_state_trees: vec![
                StateMerkleTreeAccountsV2 {
                    merkle_tree: pubkey!("bmt1LryLZUMmF7ZtqESaw7wifBXLfXHQYoE4GAmrahU"),
                    output_queue: pubkey!("oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto"),
                    cpi_context: pubkey!("cpi15BoVPKgEPw5o8wc2T816GE7b378nMXnhH3Xbq4y"),
                },
                StateMerkleTreeAccountsV2 {
                    merkle_tree: pubkey!("bmt2UxoBxB9xWev4BkLvkGdapsz6sZGkzViPNph7VFi"),
                    output_queue: pubkey!("oq2UkeMsJLfXt2QHzim242SUi3nvjJs8Pn7Eac9H9vg"),
                    cpi_context: pubkey!("cpi2yGapXUR3As5SjnHBAVvmApNiLsbeZpF3euWnW6B"),
                },
                StateMerkleTreeAccountsV2 {
                    merkle_tree: pubkey!("bmt3ccLd4bqSVZVeCJnH1F6C8jNygAhaDfxDwePyyGb"),
                    output_queue: pubkey!("oq3AxjekBWgo64gpauB6QtuZNesuv19xrhaC1ZM1THQ"),
                    cpi_context: pubkey!("cpi3mbwMpSX8FAGMZVP85AwxqCaQMfEk9Em1v8QK9Rf"),
                },
                StateMerkleTreeAccountsV2 {
                    merkle_tree: pubkey!("bmt4d3p1a4YQgk9PeZv5s4DBUmbF5NxqYpk9HGjQsd8"),
                    output_queue: pubkey!("oq4ypwvVGzCUMoiKKHWh4S1SgZJ9vCvKpcz6RT6A8dq"),
                    cpi_context: pubkey!("cpi4yyPDc4bCgHAnsenunGA8Y77j3XEDyjgfyCKgcoc"),
                },
                StateMerkleTreeAccountsV2 {
                    merkle_tree: pubkey!("bmt5yU97jC88YXTuSukYHa8Z5Bi2ZDUtmzfkDTA2mG2"),
                    output_queue: pubkey!("oq5oh5ZR3yGomuQgFduNDzjtGvVWfDRGLuDVjv9a96P"),
                    cpi_context: pubkey!("cpi5ZTjdgYpZ1Xr7B1cMLLUE81oTtJbNNAyKary2nV6"),
                },
            ],
            v2_address_trees: vec![pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx")],
            #[cfg(feature = "devenv")]
            funding_pool_config: FundingPoolConfig::get_v1(),
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
            },
            v1_state_trees: self.v1_state_trees.clone(),
            v1_address_trees: self.v1_address_trees.clone(),
            v2_state_trees: self.v2_state_trees.clone(),
            v2_address_trees: self.v2_address_trees.clone(),
            #[cfg(feature = "devenv")]
            funding_pool_config: self.funding_pool_config,
        }
    }
}
