use anchor_compressed_token::TokenData as AnchorTokenData;
use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    compressed_account::{CompressedAccount, CompressedAccountData},
    instruction_data::{
        data::OutputCompressedAccountWithPackedContext,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
    Pubkey,
};
use light_compressed_token::{
    constants::{
        TOKEN_COMPRESSED_ACCOUNT_V2_DISCRIMINATOR, TOKEN_COMPRESSED_ACCOUNT_V3_DISCRIMINATOR,
    },
    shared::{
        cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
        },
        token_output::set_output_compressed_account,
    },
};
use light_token_interface::{
    hash_cache::HashCache,
    instructions::extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
    state::{
        CompressedOnlyExtension, CompressedTokenAccountState as AccountState, ExtensionStruct,
        ExtensionStructConfig, TokenData, TokenDataConfig,
    },
};
use light_hasher::Hasher;
use light_zero_copy::{traits::ZeroCopyAt, ZeroCopyNew};

#[test]
fn test_rnd_create_output_compressed_accounts() {
    use rand::Rng;
    let mut rng = rand::rngs::ThreadRng::default();

    let iter = 1000;
    for _ in 0..iter {
        let mint_pubkey = Pubkey::new_from_array(rng.gen::<[u8; 32]>());

        // Random number of output accounts (0-30 max)
        let num_outputs = rng.gen_range(0..=30);

        // Generate random owners and amounts
        let mut owner_pubkeys = Vec::new();
        let mut amounts = Vec::new();
        let mut delegate_flags = Vec::new();
        let mut lamports_vec = Vec::new();
        let mut merkle_tree_indices = Vec::new();
        let mut tlv_flags = Vec::new();
        let mut tlv_delegated_amounts = Vec::new();
        let mut tlv_withheld_fees = Vec::new();

        for _ in 0..num_outputs {
            owner_pubkeys.push(Pubkey::new_from_array(rng.gen::<[u8; 32]>()));
            amounts.push(rng.gen_range(1..=u64::MAX));
            delegate_flags.push(rng.gen_bool(0.3)); // 30% chance of having delegate
            lamports_vec.push(if rng.gen_bool(0.2) {
                Some(rng.gen_range(1..=1000000))
            } else {
                None
            });
            merkle_tree_indices.push(rng.gen_range(0..=255u8));
            tlv_flags.push(rng.gen_bool(0.3)); // 30% chance of having TLV
            tlv_delegated_amounts.push(rng.gen_range(0..=u64::MAX));
            tlv_withheld_fees.push(rng.gen_range(0..=u64::MAX));
        }

        // Random delegate
        let delegate = if delegate_flags.iter().any(|&has_delegate| has_delegate) {
            Some(Pubkey::new_from_array(rng.gen::<[u8; 32]>()))
        } else {
            None
        };

        let lamports = if lamports_vec.iter().any(|l| l.is_some()) {
            Some(lamports_vec.clone())
        } else {
            None
        };

        // Create output config with proper TLV sizes
        let mut outputs = tinyvec::ArrayVec::<[(bool, u32); 35]>::new();
        for i in 0..num_outputs {
            let tlv_config = if tlv_flags[i] {
                vec![ExtensionStructConfig::CompressedOnly(())]
            } else {
                vec![]
            };
            let token_config = TokenDataConfig {
                delegate: (delegate_flags[i], ()),
                tlv: (!tlv_config.is_empty(), tlv_config),
            };
            let data_len = TokenData::byte_len(&token_config).unwrap() as u32;
            outputs.push((false, data_len)); // Token accounts don't have addresses
        }

        let config_input = CpiConfigInput {
            input_accounts: tinyvec::ArrayVec::<[bool; 8]>::new(),
            output_accounts: outputs,
            has_proof: false,
            new_address_params: 0,
        };

        let config = cpi_bytes_config(config_input.clone());
        let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config).unwrap();
        let (mut cpi_instruction_struct, _) = InstructionDataInvokeCpiWithReadOnly::new_zero_copy(
            &mut cpi_bytes[8..],
            config.clone(),
        )
        .unwrap();

        // Create TLV instruction data for each output
        let mut tlv_instruction_data_vecs: Vec<Vec<ExtensionInstructionData>> = Vec::new();
        let mut tlv_bytes_vecs: Vec<Vec<u8>> = Vec::new();

        for i in 0..num_outputs {
            if tlv_flags[i] {
                let ext = ExtensionInstructionData::CompressedOnly(
                    CompressedOnlyExtensionInstructionData {
                        delegated_amount: tlv_delegated_amounts[i],
                        withheld_transfer_fee: tlv_withheld_fees[i],
                        is_frozen: rng.gen_bool(0.2), // 20% chance of frozen
                        compression_index: i as u8,
                        is_ata: false,
                        bump: 0,
                        owner_index: 0,
                    },
                );
                tlv_instruction_data_vecs.push(vec![ext.clone()]);
                tlv_bytes_vecs.push(vec![ext].try_to_vec().unwrap());
            } else {
                tlv_instruction_data_vecs.push(vec![]);
                // Empty vec needs explicit type annotation and borsh serialization
                let empty_vec: Vec<ExtensionInstructionData> = vec![];
                tlv_bytes_vecs.push(empty_vec.try_to_vec().unwrap());
            }
        }

        // Parse TLV bytes to zero-copy for set_output_compressed_account calls
        let tlv_zero_copy_vecs: Vec<_> = tlv_bytes_vecs
            .iter()
            .map(|bytes| {
                Vec::<ExtensionInstructionData>::zero_copy_at(bytes.as_slice())
                    .unwrap()
                    .0
            })
            .collect();

        let mut hash_cache = HashCache::new();
        for (index, output_account) in cpi_instruction_struct
            .output_compressed_accounts
            .iter_mut()
            .enumerate()
        {
            let output_delegate = if delegate_flags[index] {
                delegate
            } else {
                None
            };

            // Use version 3 when TLV is present, version 2 otherwise
            let version = if tlv_flags[index] { 3 } else { 2 };

            // Get TLV data slice (empty slice if no TLV)
            let tlv_slice = if tlv_flags[index] && !tlv_zero_copy_vecs[index].is_empty() {
                Some(tlv_zero_copy_vecs[index].as_slice())
            } else {
                None
            };

            set_output_compressed_account(
                output_account,
                &mut hash_cache,
                owner_pubkeys[index],
                output_delegate,
                amounts[index],
                lamports.as_ref().and_then(|l| l[index]),
                mint_pubkey,
                merkle_tree_indices[index],
                version,
                tlv_slice,
                false, // Not frozen in tests
            )
            .unwrap();
        }

        let cpi_borsh =
            InstructionDataInvokeCpiWithReadOnly::deserialize(&mut &cpi_bytes[8..]).unwrap();

        // Build expected output
        let mut expected_accounts = Vec::new();

        for i in 0..num_outputs {
            let token_delegate = if delegate_flags[i] { delegate } else { None };
            let account_lamports = lamports_vec[i].unwrap_or(0);

            // Build TLV if flag is set
            let tlv = if tlv_flags[i] {
                Some(vec![ExtensionStruct::CompressedOnly(
                    CompressedOnlyExtension {
                        delegated_amount: tlv_delegated_amounts[i],
                        withheld_transfer_fee: tlv_withheld_fees[i],
                        is_ata: 0,
                    },
                )])
            } else {
                None
            };

            let token_data = AnchorTokenData {
                mint: mint_pubkey,
                owner: owner_pubkeys[i],
                amount: amounts[i],
                delegate: token_delegate,
                state: AccountState::Initialized as u8,
                tlv: tlv.clone(),
            };

            // Use V3 hash (SHA256 of serialized data) when TLV present, V2 hash otherwise
            let (data_hash, discriminator) = if tlv_flags[i] {
                let serialized = token_data.try_to_vec().unwrap();
                let hash = light_hasher::sha256::Sha256BE::hash(&serialized).unwrap();
                (hash, TOKEN_COMPRESSED_ACCOUNT_V3_DISCRIMINATOR)
            } else {
                (
                    token_data.hash_v2().unwrap(),
                    TOKEN_COMPRESSED_ACCOUNT_V2_DISCRIMINATOR,
                )
            };

            expected_accounts.push(OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    address: None,
                    owner: light_compressed_token::ID.into(),
                    lamports: account_lamports,
                    data: Some(CompressedAccountData {
                        data: token_data.try_to_vec().unwrap(),
                        discriminator,
                        data_hash,
                    }),
                },
                merkle_tree_index: merkle_tree_indices[i],
            });
        }

        let expected = InstructionDataInvokeCpiWithReadOnly {
            output_compressed_accounts: expected_accounts,
            ..Default::default()
        };
        assert_eq!(cpi_borsh, expected);
    }
}
