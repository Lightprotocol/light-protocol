// #![cfg(feature = "test-sbf")]

mod event;

use event::get_compressed_output_account;
use light_program_test::test_env::setup_test_programs_with_accounts;
use light_prover_client::gnark::helpers::{spawn_prover, ProverConfig, ProverMode};
use light_test_utils::RpcConnection;
#[tokio::test]
async fn functional_read_only() {
    let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("create_address_test_program"),
        create_address_test_program::ID,
    )]))
    .await;
    spawn_prover(
        false,
        ProverConfig {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        },
    )
    .await;

    let payer = rpc.get_payer().insecure_clone();

    let output_accounts = vec![get_compressed_output_account(true, env.batched_output_queue,); 8];
    local_sdk::perform_test_transaction(
        &mut rpc,
        &payer,
        vec![],
        output_accounts,
        vec![],
        None,
        None,
        true,
    )
    .await
    .unwrap();
}

pub mod local_sdk {
    use std::collections::HashMap;

    use anchor_lang::{prelude::AccountMeta, AnchorSerialize};

    use create_address_test_program::create_invoke_read_only_account_info_instruction;
    use light_compressed_account::address::pack_new_address_params_assigned;
    use light_compressed_account::compressed_account::{
        pack_compressed_accounts, pack_output_compressed_accounts,
        CompressedAccountWithMerkleContext, PackedCompressedAccountWithMerkleContext,
    };
    use light_compressed_account::indexer_event::event::BatchPublicTransactionEvent;
    use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
    use light_compressed_account::instruction_data::with_readonly::{
        InAccount, InstructionDataInvokeCpiWithReadOnly,
    };
    use light_compressed_token::process_transfer::transfer_sdk::to_account_metas;
    use light_sdk::{cpi::accounts::SystemAccountPubkeys, find_cpi_signer_macro};
    use light_sdk::{
        NewAddressParamsAssigned, OutputCompressedAccountWithContext,
        OutputCompressedAccountWithPackedContext, CPI_AUTHORITY_PDA_SEED,
    };
    use light_test_utils::{RpcConnection, RpcError};
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::{Keypair, Signer};

    pub async fn perform_test_transaction<R: RpcConnection>(
        rpc: &mut R,
        payer: &Keypair,
        input_accounts: Vec<CompressedAccountWithMerkleContext>,
        output_accounts: Vec<OutputCompressedAccountWithContext>,
        new_addresses: Vec<NewAddressParamsAssigned>,
        proof: Option<CompressedProof>,
        config: Option<SystemAccountMetaConfig>,
        read_only: bool,
    ) -> Result<
        Option<(
            Vec<BatchPublicTransactionEvent>,
            Vec<OutputCompressedAccountWithPackedContext>,
            Vec<PackedCompressedAccountWithMerkleContext>,
        )>,
        RpcError,
    > {
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();

        let packed_new_address_params =
            pack_new_address_params_assigned(new_addresses.as_slice(), &mut remaining_accounts);

        let packed_inputs = pack_compressed_accounts(
            input_accounts.as_slice(),
            &vec![None; input_accounts.len()],
            &mut remaining_accounts,
        );
        let output_compressed_accounts = pack_output_compressed_accounts(
            output_accounts
                .iter()
                .map(|x| x.compressed_account.clone())
                .collect::<Vec<_>>()
                .as_slice(),
            output_accounts
                .iter()
                .map(|x| x.merkle_tree)
                .collect::<Vec<_>>()
                .as_slice(),
            &mut remaining_accounts,
        );

        let ix_data = InstructionDataInvokeCpiWithReadOnly {
            mode: 0,
            bump: 255,
            with_cpi_context: false,
            invoking_program_id: create_address_test_program::ID.into(),
            proof,
            new_address_params: packed_new_address_params,
            is_decompress: false,
            compress_or_decompress_lamports: 0,
            output_compressed_accounts: output_compressed_accounts.clone(),
            input_compressed_accounts: packed_inputs
                .iter()
                .map(|x| InAccount {
                    address: x.compressed_account.address,
                    merkle_context: x.merkle_context,
                    lamports: x.compressed_account.lamports,
                    discriminator: x.compressed_account.data.as_ref().unwrap().discriminator,
                    data_hash: x.compressed_account.data.as_ref().unwrap().data_hash,
                    root_index: x.root_index,
                })
                .collect::<Vec<_>>(),
            with_transaction_hash: true,
            ..Default::default()
        };
        let remaining_accounts = to_account_metas(remaining_accounts);
        let config = if let Some(config) = config {
            config
        } else {
            SystemAccountMetaConfig {
                self_program: create_address_test_program::ID,
                cpi_context: None,
                sol_pool_pda: None,
                sol_compression_recipient: None,
                small_ix: false,
            }
        };
        let instruction_discriminator = if read_only {
            // INVOKE_CPI_WITH_READ_ONLY_INSTRUCTIOM
            [86, 47, 163, 166, 21, 223, 92, 8, 0, 0, 0, 0]
        } else {
            [228, 34, 128, 84, 47, 139, 86, 240, 0, 0, 0, 0]
            // INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION
        };
        let remaining_accounts =
            [get_light_system_account_metas(config), remaining_accounts].concat();
        let instruction = create_invoke_read_only_account_info_instruction(
            payer.pubkey(),
            [
                instruction_discriminator.to_vec(),
                ix_data.try_to_vec().unwrap(),
            ]
            .concat(),
            remaining_accounts,
        );
        let res = rpc
            .create_and_send_transaction_with_batched_event(
                &[instruction],
                &payer.pubkey(),
                &[payer],
                None,
            )
            .await?;
        if let Some(res) = res {
            Ok(Some((res.0, output_compressed_accounts, packed_inputs)))
        } else {
            Ok(None)
        }
    }

    // Offchain
    #[derive(Debug, Default, Copy, Clone)]
    pub struct SystemAccountMetaConfig {
        pub self_program: Pubkey,
        pub cpi_context: Option<Pubkey>,
        pub sol_compression_recipient: Option<Pubkey>,
        pub sol_pool_pda: Option<Pubkey>,
        /// None means use regular instruction.
        /// Some means use instruction small.
        pub small_ix: bool,
    }

    impl SystemAccountMetaConfig {
        pub fn new(self_program: Pubkey) -> Self {
            Self {
                self_program,
                cpi_context: None,
                sol_compression_recipient: None,
                sol_pool_pda: None,
                small_ix: false,
            }
        }
        pub fn new_with_account_options(self_program: Pubkey) -> Self {
            Self {
                self_program,
                cpi_context: None,
                sol_compression_recipient: None,
                sol_pool_pda: None,
                small_ix: true,
            }
        }

        pub fn new_with_cpi_context(self_program: Pubkey, cpi_context: Pubkey) -> Self {
            Self {
                self_program,
                cpi_context: Some(cpi_context),
                sol_compression_recipient: None,
                sol_pool_pda: None,
                small_ix: false,
            }
        }
    }

    #[derive(Default, Debug)]
    pub struct PackedAccounts {
        pre_accounts: Vec<AccountMeta>,
        system_accounts: Vec<AccountMeta>,
        next_index: u8,
        map: HashMap<Pubkey, (u8, AccountMeta)>,
    }

    impl PackedAccounts {
        pub fn new_with_system_accounts(config: SystemAccountMetaConfig) -> Self {
            let mut remaining_accounts = PackedAccounts::default();
            remaining_accounts.add_system_accounts(config);
            remaining_accounts
        }

        pub fn add_pre_accounts_signer(&mut self, pubkey: Pubkey) {
            self.pre_accounts.push(AccountMeta {
                pubkey,
                is_signer: true,
                is_writable: false,
            });
        }

        pub fn add_pre_accounts_signer_mut(&mut self, pubkey: Pubkey) {
            self.pre_accounts.push(AccountMeta {
                pubkey,
                is_signer: true,
                is_writable: true,
            });
        }

        pub fn add_pre_accounts_meta(&mut self, account_meta: AccountMeta) {
            self.pre_accounts.push(account_meta);
        }

        pub fn add_system_accounts(&mut self, config: SystemAccountMetaConfig) {
            self.system_accounts
                .extend(get_light_system_account_metas(config));
        }

        /// Returns the index of the provided `pubkey` in the collection.
        ///
        /// If the provided `pubkey` is not a part of the collection, it gets
        /// inserted with a `next_index`.
        ///
        /// If the privided `pubkey` already exists in the collection, its already
        /// existing index is returned.
        pub fn insert_or_get(&mut self, pubkey: Pubkey) -> u8 {
            self.insert_or_get_config(pubkey, false, true)
        }

        pub fn insert_or_get_read_only(&mut self, pubkey: Pubkey) -> u8 {
            self.insert_or_get_config(pubkey, false, false)
        }

        pub fn insert_or_get_config(
            &mut self,
            pubkey: Pubkey,
            is_signer: bool,
            is_writable: bool,
        ) -> u8 {
            self.map
                .entry(pubkey)
                .or_insert_with(|| {
                    let index = self.next_index;
                    self.next_index += 1;
                    (
                        index,
                        AccountMeta {
                            pubkey,
                            is_signer,
                            is_writable,
                        },
                    )
                })
                .0
        }

        fn hash_set_accounts_to_metas(&self) -> Vec<AccountMeta> {
            let mut packed_accounts = self.map.iter().collect::<Vec<_>>();
            // hash maps are not sorted so we need to sort manually and collect into a vector again
            packed_accounts.sort_by(|a, b| a.1 .0.cmp(&b.1 .0));
            let packed_accounts = packed_accounts
                .iter()
                .map(|(_, (_, k))| k.clone())
                .collect::<Vec<AccountMeta>>();
            packed_accounts
        }

        fn get_offsets(&self) -> (usize, usize) {
            let system_accounts_start_offset = self.pre_accounts.len();
            let packed_accounts_start_offset =
                system_accounts_start_offset + self.system_accounts.len();
            (system_accounts_start_offset, packed_accounts_start_offset)
        }

        /// Converts the collection of accounts to a vector of
        /// [`AccountMeta`](solana_sdk::instruction::AccountMeta), which can be used
        /// as remaining accounts in instructions or CPI calls.
        pub fn to_account_metas(&self) -> (Vec<AccountMeta>, usize, usize) {
            let packed_accounts = self.hash_set_accounts_to_metas();
            let (system_accounts_start_offset, packed_accounts_start_offset) = self.get_offsets();
            (
                [
                    self.pre_accounts.clone(),
                    self.system_accounts.clone(),
                    packed_accounts,
                ]
                .concat(),
                system_accounts_start_offset,
                packed_accounts_start_offset,
            )
        }
    }

    pub fn get_light_system_account_metas(config: SystemAccountMetaConfig) -> Vec<AccountMeta> {
        let cpi_signer = find_cpi_signer_macro!(&config.self_program).0;
        let default_pubkeys = SystemAccountPubkeys::default();
        let mut vec = if config.small_ix {
            let vec = vec![
                AccountMeta::new_readonly(cpi_signer, false),
                AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
                AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
                AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
            ];
            vec
        } else {
            let vec = vec![
                AccountMeta::new_readonly(default_pubkeys.light_sytem_program, false),
                AccountMeta::new_readonly(cpi_signer, false),
                AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
                AccountMeta::new_readonly(default_pubkeys.noop_program, false),
                AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
                AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
                AccountMeta::new_readonly(config.self_program, false),
            ];

            vec
        };
        if let Some(pubkey) = config.sol_pool_pda {
            vec.push(AccountMeta {
                pubkey,
                is_signer: false,
                is_writable: true,
            });
        }
        if let Some(pubkey) = config.sol_compression_recipient {
            vec.push(AccountMeta {
                pubkey,
                is_signer: false,
                is_writable: true,
            });
        }
        if !config.small_ix {
            vec.push(AccountMeta::new_readonly(
                default_pubkeys.system_program,
                false,
            ));
        }
        if let Some(pubkey) = config.cpi_context {
            vec.push(AccountMeta {
                pubkey,
                is_signer: false,
                is_writable: true,
            });
        }
        vec
    }
}
