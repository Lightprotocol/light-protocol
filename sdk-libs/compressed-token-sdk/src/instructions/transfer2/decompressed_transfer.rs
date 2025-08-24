use crate::{
    account2::CTokenAccount2, error::Result,
    instructions::transfer2::account_metas::Transfer2AccountsMetaConfig,
};
use light_compressed_token_types::ValidityProof;
use light_ctoken_types::{
    instructions::transfer2::{Compression, CompressionMode, MultiTokenTransferOutputData},
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use super::instruction::{create_transfer2_instruction, Transfer2Config, Transfer2Inputs};

pub fn create_spl_to_ctoken_transfer_instruction(
    source_spl_token_account: Pubkey,
    to: Pubkey,
    amount: u64,
    authority: Pubkey,
    mint: Pubkey,
    payer: Pubkey,
    token_pool_pda: Pubkey,
) -> Result<Instruction> {
    let mut packed_accounts = Vec::new();

    // Tree account placeholer (index 0)
    packed_accounts.push(AccountMeta::new(Pubkey::new_from_array([2; 32]), false));

    // Mint (index 1)
    packed_accounts.push(AccountMeta::new_readonly(mint, false));

    // Destination owner (index 2) // or to??
    packed_accounts.push(AccountMeta::new_readonly(to, false));

    // Authority for compression (index 3) - signer
    packed_accounts.push(AccountMeta::new_readonly(authority, true));

    // Source SPL token account (index 4) - writable
    packed_accounts.push(AccountMeta::new(source_spl_token_account, false));

    // Token pool PDA (index 5) - writable
    packed_accounts.push(AccountMeta::new(token_pool_pda, false));

    let (derived_token_pool_pda, bump) = Pubkey::find_program_address(
        &[
            b"token_pool",
            mint.as_ref(),
            &0u8.to_le_bytes(), // Usually 0
        ],
        &COMPRESSED_TOKEN_PROGRAM_ID.into(),
    );
    msg!("bump: {:?}", bump);
    if derived_token_pool_pda != token_pool_pda {
        msg!(
            "Token pool PDA mismatch: {:?}, {:?}",
            derived_token_pool_pda,
            token_pool_pda
        );
        panic!("Token pool PDA mismatch");
    }
    let ctoken_account = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData {
            owner: 2, // recipient of output
            amount: 0,
            merkle_tree: 0,
            delegate: 0,
            mint: 1,
            version: 0,
        },
        compression: Some(Compression {
            amount,
            mode: CompressionMode::Compress,
            mint: 1,
            source_or_recipient: 4, // index of source
            authority: 3,
            pool_account_index: 5,
            pool_index: 0, // TODO: make dynamic.
            bump,
        }),
        delegate_is_set: false,
        method_used: true,
    };

    // Create Transfer2Inputs following the test pattern
    let inputs = Transfer2Inputs {
        validity_proof: ValidityProof::default(),
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config: Transfer2AccountsMetaConfig::new(payer, packed_accounts),
        in_lamports: None,
        out_lamports: None,
        token_accounts: vec![ctoken_account],
    };

    // Create the actual transfer2 instruction
    create_transfer2_instruction(inputs)
}

/// Transfer SPL tokens to compressed tokens
///
/// This function creates the instruction and immediately invokes it.
/// Similar to SPL Token's transfer wrapper functions.
pub fn transfer_spl_to_ctoken<'info>(
    from: &'info AccountInfo<'info>,
    to: &'info AccountInfo<'info>,
    amount: u64,
    authority: &'info AccountInfo<'info>,
    mint: &'info AccountInfo<'info>,
    payer: &'info AccountInfo<'info>,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> std::result::Result<(), ProgramError> {
    // Validate minimum accounts required
    if remaining_accounts.len() < 8 {
        return Err(ProgramError::InvalidAccountData);
    }

    // Derive token pool PDA
    let (token_pool_pda, _bump) = Pubkey::find_program_address(
        &[b"token_pool", mint.key.as_ref(), &0u8.to_le_bytes()],
        &COMPRESSED_TOKEN_PROGRAM_ID.into(),
    );

    // Find the token pool account in remaining accounts
    let token_pool_account = remaining_accounts
        .iter()
        .find(|acc| acc.key == &token_pool_pda)
        .ok_or(ProgramError::InvalidAccountData)?;

    // Create the instruction
    let instruction = create_spl_to_ctoken_transfer_instruction(
        *from.key,
        *to.key,
        amount,
        *authority.key,
        *mint.key,
        *payer.key,
        token_pool_pda,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Build account infos and invoke
    let account_infos = vec![
        remaining_accounts[0].clone(), // light_system_program
        payer.clone(),
        remaining_accounts[2].clone(), // cpi_authority_pda
        remaining_accounts[3].clone(), // registered_program_pda
        remaining_accounts[4].clone(), // account_compression_authority
        remaining_accounts[5].clone(), // account_compression_program
        remaining_accounts[6].clone(), // system_program
        // Packed accounts (matching the order in create_spl_to_ctoken_transfer_instruction)
        remaining_accounts[7].clone(), // placeholder_tree_account (not used for decompressed)
        mint.clone(),                  // Index 1: Mint
        to.clone(),                    // Index 2: Destination owner
        authority.clone(),             // Index 3: Authority (signer)
        from.clone(),                  // Index 4: Source SPL token account
        token_pool_account.clone(),    // Index 5: Token pool PDA
    ];

    invoke(&instruction, &account_infos)
}

/// Transfer SPL tokens to compressed tokens with program signature
///
/// This function creates the instruction and invokes it with signer seeds.
/// Used when calling from a program that needs to sign as a PDA.
pub fn transfer_spl_to_ctoken_signed<'info>(
    from: &'info AccountInfo<'info>,
    to: &'info AccountInfo<'info>,
    amount: u64,
    authority: &'info AccountInfo<'info>,
    mint: &'info AccountInfo<'info>,
    payer: &'info AccountInfo<'info>,
    remaining_accounts: &'info [AccountInfo<'info>],
    signers_seeds: &[&[&[u8]]],
) -> std::result::Result<(), ProgramError> {
    // Validate minimum accounts required
    if remaining_accounts.len() < 8 {
        return Err(ProgramError::InvalidAccountData);
    }

    // Derive token pool PDA
    let (token_pool_pda, _bump) = Pubkey::find_program_address(
        &[b"token_pool", mint.key.as_ref(), &0u8.to_le_bytes()],
        &COMPRESSED_TOKEN_PROGRAM_ID.into(),
    );

    // Find the token pool account in remaining accounts
    let token_pool_account = remaining_accounts
        .iter()
        .find(|acc| acc.key == &token_pool_pda)
        .ok_or(ProgramError::InvalidAccountData)?;

    // Create the instruction
    let instruction = create_spl_to_ctoken_transfer_instruction(
        *from.key,
        *to.key,
        amount,
        *authority.key,
        *mint.key,
        *payer.key,
        token_pool_pda,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Build account infos and invoke with signer seeds
    let account_infos = vec![
        remaining_accounts[0].clone(), // light_system_program
        payer.clone(),
        remaining_accounts[2].clone(), // cpi_authority_pda
        remaining_accounts[3].clone(), // registered_program_pda
        remaining_accounts[4].clone(), // account_compression_authority
        remaining_accounts[5].clone(), // account_compression_program
        remaining_accounts[6].clone(), // system_program
        // Packed accounts (matching the order in create_spl_to_ctoken_transfer_instruction)
        remaining_accounts[7].clone(), // placeholder_tree_account (not used for decompressed)
        mint.clone(),                  // Index 1: Mint
        to.clone(),                    // Index 2: Destination owner
        authority.clone(),             // Index 3: Authority (signer)
        from.clone(),                  // Index 4: Source SPL token account
        token_pool_account.clone(),    // Index 5: Token pool PDA
    ];

    invoke_signed(&instruction, &account_infos, signers_seeds)
}
