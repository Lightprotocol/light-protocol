use light_ctoken_types::{
    instructions::transfer2::MultiInputTokenDataWithContext, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_sdk::{compressible::create_or_allocate_account, AnchorDeserialize, AnchorSerialize};
use solana_account_info::AccountInfo;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::error::Result;

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct PackedCompressedTokenDataWithContext {
    pub mint: u8,
    pub source_or_recipient_token_account: u8,
    pub multi_input_token_data_with_context: MultiInputTokenDataWithContext,
}

/// Input parameters for creating a token account with compressible extension
#[derive(Debug, Clone)]
pub struct CreateCompressibleTokenAccount {
    /// The account to be created
    pub account_pubkey: Pubkey,
    /// The mint for the token account
    pub mint_pubkey: Pubkey,
    /// The owner of the token account
    pub owner_pubkey: Pubkey,
    /// The authority that can close this account (in addition to owner)
    pub rent_authority: Pubkey,
    /// The recipient of lamports when the account is closed by rent authority
    pub rent_recipient: Pubkey,
    /// Number of slots that must pass before compression is allowed
    pub slots_until_compression: u64,
}

pub fn initialize_compressible_token_account(
    inputs: CreateCompressibleTokenAccount,
) -> Result<Instruction> {
    // Format: [18, owner_pubkey_32_bytes, 0]
    // Create compressible extension data manually
    // Layout: [slots_until_compression: u64, rent_authority: 32 bytes, rent_recipient: 32 bytes]
    let mut data = Vec::with_capacity(1 + 32 + 1 + 8 + 32 + 32);
    data.push(18u8); // InitializeAccount3 opcode
    data.extend_from_slice(&inputs.owner_pubkey.to_bytes());
    data.push(1); // Some option byte extension
    data.extend_from_slice(&inputs.slots_until_compression.to_le_bytes());
    data.extend_from_slice(&inputs.rent_authority.to_bytes());
    data.extend_from_slice(&inputs.rent_recipient.to_bytes());

    Ok(Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: vec![
            solana_instruction::AccountMeta::new(inputs.account_pubkey, false),
            solana_instruction::AccountMeta::new_readonly(inputs.mint_pubkey, false),
        ],
        data,
    })
}

#[cfg(feature = "anchor")]
pub fn create_compressible_token_account<'a>(
    authority: &AccountInfo<'a>,
    payer: &AccountInfo<'a>,
    token_account: &AccountInfo<'a>,
    mint_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    signer_seeds: &[&[u8]],
    rent_authority: &AccountInfo<'a>,
    rent_recipient: &AccountInfo<'a>,
    slots_until_compression: u64,
) -> std::result::Result<(), solana_program_error::ProgramError> {
    use anchor_lang::ToAccountInfo;
    use solana_cpi::invoke;

    let space = COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize;

    create_or_allocate_account(
        token_program.key,
        payer.to_account_info(),
        system_program.to_account_info(),
        token_account.to_account_info(),
        signer_seeds,
        space,
    )?;

    let init_ix = initialize_compressible_token_account(CreateCompressibleTokenAccount {
        account_pubkey: *token_account.key,
        mint_pubkey: *mint_account.key,
        owner_pubkey: *authority.key,
        rent_authority: *rent_authority.key,
        rent_recipient: *rent_recipient.key,
        slots_until_compression,
    })?;

    invoke(
        &init_ix,
        &[
            token_account.to_account_info(),
            mint_account.to_account_info(),
            authority.to_account_info(),
            rent_authority.to_account_info(),
            rent_recipient.to_account_info(),
        ],
    )?;

    Ok(())
}
