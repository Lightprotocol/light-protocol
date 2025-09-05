use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountIterator;
use light_ctoken_types::{state::get_rent, BASE_TOKEN_ACCOUNT_SIZE};
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use super::instruction_data::CreateTokenAccountInstructionData;
use crate::shared::{
    create_pda_account, initialize_token_account::initialize_token_account, CreatePdaAccountConfig,
};

/// Process the create token account instruction
pub fn process_create_token_account(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let mut padded_instruction_data = [0u8; 33];
    let (inputs, _) = if instruction_data.len() == 32 {
        // Extend instruction data with a zero option byte for initialize_3 spl_token instruction compatibility
        padded_instruction_data[0..32].copy_from_slice(instruction_data);
        CreateTokenAccountInstructionData::zero_copy_at(padded_instruction_data.as_slice())
            .map_err(ProgramError::from)?
    } else {
        CreateTokenAccountInstructionData::zero_copy_at(instruction_data)
            .map_err(ProgramError::from)?
    };

    let mut iter = AccountIterator::new(account_infos);
    let token_account = iter.next_mut("token_account")?;
    // Mint is not required and not used.
    // Mint is specified for compatibility with solana.
    // TODO: provide either mint or decimals. our trick with pubkey derivation based on mint
    // only works for compressed not for spl mints.
    let mint: &AccountInfo = iter.next_non_mut("mint")?;
    // Create account via cpi
    if let Some(compressible_config) = inputs.compressible_config.as_ref() {
        // Not os solana we assume that the accoun already exists and just transfer funds
        let payer = iter.next_signer_mut("payer")?;
        #[cfg(target_os = "solana")]
        {
            let system_program = iter.next_account("system program")?;
            // Check derivation
            // payer pda pays for account creation
            let seeds2 = [b"pool".as_slice()];
            let derived_pool_pda = pinocchio_pubkey::derive_address(
                &seeds2,
                Some(compressible_config.payer_pda_bump),
                crate::ID.as_array(),
            );
            // TODO: also compare the rent recipient and rent authority
            let config = if compressible_config.has_rent_recipient != 0
                && compressible_config.rent_authority == derived_pool_pda
                && compressible_config.rent_recipient == derived_pool_pda
            {
                CreatePdaAccountConfig {
                    seeds: seeds2.as_slice(),
                    bump: compressible_config.payer_pda_bump,
                    account_size: BASE_TOKEN_ACCOUNT_SIZE as usize + 96,
                    owner_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
                    derivation_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
                }
            } else {
                return Err(ProgramError::InvalidInstructionData);
            };

            create_pda_account(payer, token_account, system_program, config, None)?;
        }

        let rent = get_rent(
            token_account.data_len() as u64,
            compressible_config.rent_payment.get(),
        );
        transfer_lamports(rent, payer, token_account)?;
    }

    // Initialize the token account (assumes account already exists and is owned by our program)
    initialize_token_account(
        token_account,
        mint.key(),
        &inputs.owner.to_bytes(),
        inputs.compressible_config,
    )?;

    Ok(())
}

pub fn transfer_lamports(
    amount: u64,
    from: &AccountInfo,
    to: &AccountInfo,
) -> Result<(), ProgramError> {
    let from_lamports: u64 = *from
        .try_borrow_lamports()
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
    let to_lamports: u64 = *to
        .try_borrow_lamports()
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;

    if from_lamports < amount {
        msg!("payer lamports {}", from_lamports);
        msg!("required lamports {}", amount);
        return Err(ProgramError::InsufficientFunds);
    }

    let from_lamports = from_lamports
        .checked_sub(amount)
        .ok_or(ProgramError::InsufficientFunds)?;
    let to_lamports = to_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InsufficientFunds)?;
    *from
        .try_borrow_mut_lamports()
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))? = from_lamports;
    *to.try_borrow_mut_lamports()
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))? = to_lamports;
    Ok(())
}

#[cfg(test)]
pub mod test {
    use std::sync::atomic::AtomicU64;

    pub static CURRENT_SLOT: AtomicU64 = AtomicU64::new(0);
    use borsh::BorshSerialize;
    use light_account_checks::{
        account_info::test_account_info::pinocchio::{get_account_info, pubkey_unique},
        AccountInfoTrait,
    };
    use light_ctoken_types::{
        instructions::extensions::compressible::CompressibleExtensionInstructionData,
        state::{
            solana_ctoken::{CompressedToken, CompressedTokenConfig},
            CompressibleExtension, CompressibleExtensionConfig, ExtensionStruct,
            ExtensionStructConfig,
        },
    };
    use light_zero_copy::traits::ZeroCopyNew;
    use pinocchio::account_info::AccountInfo;
    use solana_pubkey::Pubkey;

    use super::*;
    use crate::{
        create_token_account::instruction_data::CreateTokenAccountInstructionData,
        decompressed_token_transfer::process_decompressed_token_transfer,
    };
    fn create_ctoken_account_info() -> AccountInfo {
        let config = CompressedTokenConfig {
            delegate: false,
            is_native: false,
            close_authority: false,
            extensions: vec![ExtensionStructConfig::Compressible(
                CompressibleExtensionConfig {
                    write_top_up_lamports: true,
                    rent_authority: (true, ()),
                    rent_recipient: (true, ()),
                },
            )],
        };

        let required_size = CompressedToken::byte_len(&config).unwrap();
        let buffer = vec![0u8; required_size];
        let account_info = get_account_info(
            pubkey_unique(),
            crate::COMPRESSED_TOKEN_PROGRAM_ID.into(),
            false,
            true,
            false,
            buffer,
        );
        *account_info.try_borrow_mut_lamports().unwrap() = 0;
        account_info
    }
    fn create_mint_account_info() -> AccountInfo {
        let buffer = vec![0u8; 0];
        get_account_info(
            pubkey_unique(),
            crate::COMPRESSED_TOKEN_PROGRAM_ID.into(),
            false,
            false,
            false,
            buffer,
        )
    }

    fn create_payer_account_info() -> AccountInfo {
        let buffer = vec![0u8; 0];
        let account_info = get_account_info(
            pubkey_unique(),
            crate::COMPRESSED_TOKEN_PROGRAM_ID.into(),
            true,
            true,
            false,
            buffer,
        );
        *account_info.try_borrow_mut_lamports().unwrap() = 10_000_000_000;
        account_info
    }

    #[track_caller]
    fn assert_ctoken_account(account_info: &AccountInfo, expected_account: CompressedToken) {
        let data = account_info.try_borrow_data().unwrap();
        let (ctoken_account, _) = CompressedToken::zero_copy_at(&data[..]).unwrap();
        assert_eq!(ctoken_account, expected_account);
    }

    #[track_caller]
    fn set_ctoken_account_amount(account_info: &AccountInfo, amount: u64) {
        use light_zero_copy::traits::ZeroCopyAtMut;
        let mut data = account_info.try_borrow_mut_data().unwrap();
        let (mut ctoken_account, _) = CompressedToken::zero_copy_at_mut(&mut data[..]).unwrap();
        *ctoken_account.amount = amount.into();
    }
    fn process_and_assert_ctoken_account(
        owner: light_compressed_account::Pubkey,
        ctoken_account_info: &AccountInfo,
        mint_account: &AccountInfo,
        payer_account_info: &AccountInfo,
        write_top_up: u32,
    ) {
        let rent_authority = Pubkey::new_unique().into();
        let rent_recipient = Pubkey::new_unique().into();

        let instruction_data = CreateTokenAccountInstructionData {
            owner,
            compressible_config: Some(CompressibleExtensionInstructionData {
                rent_payment: 1,
                has_rent_authority: 1,
                rent_authority,
                has_rent_recipient: 1,
                rent_recipient,
                has_top_up: 1,
                write_top_up,
                payer_pda_bump: 255,
            }),
        };

        process_create_token_account(
            &[
                ctoken_account_info.clone(),
                mint_account.clone(),
                payer_account_info.clone(),
            ],
            instruction_data.try_to_vec().unwrap().as_slice(),
        )
        .unwrap();
        let expected_account = CompressedToken {
            amount: 0,
            mint: (*mint_account.key()).into(),
            owner,
            close_authority: None,
            delegate: None,
            delegated_amount: 0,
            state: 1,
            is_native: None,
            extensions: Some(vec![ExtensionStruct::Compressible(CompressibleExtension {
                last_claimed_slot: 1,
                lamports_at_last_claimed_slot: get_rent(ctoken_account_info.data_len() as u64, 1),
                write_top_up_lamports: Some(write_top_up),
                version: 1,
                rent_authority: Some(rent_authority.to_bytes()),
                rent_recipient: Some(rent_recipient.to_bytes()),
            })]),
        };
        assert_ctoken_account(ctoken_account_info, expected_account);
    }

    #[test]
    fn test_rent() {
        let ctoken_account_info = create_ctoken_account_info();
        let mint_account = create_mint_account_info();
        let payer_account_info = create_payer_account_info();
        let payer_account_info2 = create_payer_account_info();
        let ctoken_account_info2 = create_ctoken_account_info();
        let owner = payer_account_info.pubkey().into();
        let owner2 = payer_account_info2.pubkey().into();

        process_and_assert_ctoken_account(
            owner,
            &ctoken_account_info,
            &mint_account,
            &payer_account_info,
            1,
        );
        process_and_assert_ctoken_account(
            owner2,
            &ctoken_account_info2,
            &mint_account,
            &payer_account_info2,
            2,
        );
        set_ctoken_account_amount(&ctoken_account_info, 10);
        // Transfer amount 0
        // We are just interested in the write payment.
        {
            let pre_lamports1 = ctoken_account_info.lamports().clone();
            let pre_lamports2 = ctoken_account_info2.lamports().clone();
            let mut instruction_data = [0u8; 10];
            instruction_data[1] = 3;
            instruction_data[2..].copy_from_slice(1u64.to_le_bytes().as_slice());
            process_decompressed_token_transfer(
                &[
                    ctoken_account_info.clone(),  // from
                    ctoken_account_info2.clone(), // to
                    payer_account_info.clone(),   // signer
                ],
                instruction_data.as_slice(),
            )
            .unwrap();
            let post_lamports1 = ctoken_account_info.lamports().clone();
            let post_lamports2 = ctoken_account_info2.lamports().clone();
            assert_eq!(pre_lamports1 + 1, post_lamports1);
            assert_eq!(pre_lamports2 + 2, post_lamports2);
        }
    }
}
