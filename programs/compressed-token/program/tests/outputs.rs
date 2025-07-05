use anchor_compressed_token::token_data::TokenData as AnchorTokenData;
use arrayvec::ArrayVec;
use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    compressed_account::{CompressedAccount, CompressedAccountData},
    hash_to_bn254_field_size_be,
    instruction_data::{
        data::OutputCompressedAccountWithPackedContext,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
    Pubkey,
};
use light_compressed_token::{
    constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
    shared::{
        cpi_bytes_size::{allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput},
        outputs::create_output_compressed_accounts,
    },
};
use light_zero_copy::ZeroCopyNew;

#[test]
fn test_rnd_create_output_compressed_accounts() {
    use rand::Rng;
    let mut rng = rand::rngs::ThreadRng::default();

    let iter = 1000;
    for _ in 0..iter {
        let mint_pubkey = Pubkey::new_from_array(rng.gen::<[u8; 32]>());
        let hashed_mint = hash_to_bn254_field_size_be(mint_pubkey.to_bytes().as_slice());

        // Random number of output accounts (0-35 max)
        let num_outputs = rng.gen_range(0..=35);

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

        let is_delegate = if delegate.is_some() {
            Some(delegate_flags.clone())
        } else {
            None
        };
        let lamports = if lamports_vec.iter().any(|l| l.is_some()) {
            Some(lamports_vec.clone())
        } else {
            None
        };

        // Create output config
        let mut outputs = ArrayVec::new();
        for &has_delegate in &delegate_flags {
            outputs.push(has_delegate);
        }

        let config_input = CpiConfigInput {
            input_accounts: ArrayVec::new(),
            output_accounts: outputs,
            has_proof: false,
            compressed_mint: false,
            compressed_mint_with_freeze_authority: false,
        };

        let config = cpi_bytes_config(config_input.clone());
        let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);
        let (cpi_instruction_struct, _) = InstructionDataInvokeCpiWithReadOnly::new_zero_copy(
            &mut cpi_bytes[8..],
            config.clone(),
        )
        .unwrap();

        let sum_lamports = create_output_compressed_accounts(
            cpi_instruction_struct,
            mint_pubkey,
            &owner_pubkeys,
            delegate,
            is_delegate,
            &amounts,
            lamports,
            &hashed_mint,
            &merkle_tree_indices,
        )
        .unwrap();

        let cpi_borsh =
            InstructionDataInvokeCpiWithReadOnly::deserialize(&mut &cpi_bytes[8..]).unwrap();

        // Build expected output
        let mut expected_accounts = Vec::new();
        let mut expected_sum_lamports = 0u64;

        for i in 0..num_outputs {
            let token_delegate = if delegate_flags[i] { delegate } else { None };
            let account_lamports = lamports_vec[i].unwrap_or(0);
            expected_sum_lamports += account_lamports;

            let token_data = AnchorTokenData {
                mint: mint_pubkey.into(),
                owner: owner_pubkeys[i].into(),
                amount: amounts[i],
                delegate: token_delegate.map(|d| d.into()),
                state: anchor_compressed_token::token_data::AccountState::Initialized,
                tlv: None,
            };
            let data_hash = token_data.hash().unwrap();

            expected_accounts.push(OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    address: None,
                    owner: light_compressed_token::ID.into(),
                    lamports: account_lamports,
                    data: Some(CompressedAccountData {
                        data: token_data.try_to_vec().unwrap(),
                        discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
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
        assert_eq!(sum_lamports, expected_sum_lamports);
    }
}