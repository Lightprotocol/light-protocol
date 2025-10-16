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
    constants::TOKEN_COMPRESSED_ACCOUNT_V2_DISCRIMINATOR,
    shared::{
        cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, compressed_token_data_len, cpi_bytes_config,
            CpiConfigInput,
        },
        token_output::set_output_compressed_account,
    },
};
use light_ctoken_types::{
    hash_cache::HashCache, state::CompressedTokenAccountState as AccountState,
};
use light_zero_copy::ZeroCopyNew;

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

        // Create output config
        let mut outputs = tinyvec::ArrayVec::<[(bool, u32); 35]>::new();
        for &has_delegate in &delegate_flags {
            outputs.push((false, compressed_token_data_len(has_delegate))); // Token accounts don't have addresses
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

            set_output_compressed_account(
                output_account,
                &mut hash_cache,
                owner_pubkeys[index],
                output_delegate,
                amounts[index],
                lamports.as_ref().and_then(|l| l[index]),
                mint_pubkey,
                merkle_tree_indices[index],
                2,
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

            let token_data = AnchorTokenData {
                mint: mint_pubkey,
                owner: owner_pubkeys[i],
                amount: amounts[i],
                delegate: token_delegate,
                state: AccountState::Initialized as u8,
                tlv: None,
            };
            let data_hash = token_data.hash_v2().unwrap();

            expected_accounts.push(OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    address: None,
                    owner: light_compressed_token::ID.into(),
                    lamports: account_lamports,
                    data: Some(CompressedAccountData {
                        data: token_data.try_to_vec().unwrap(),
                        discriminator: TOKEN_COMPRESSED_ACCOUNT_V2_DISCRIMINATOR,
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
