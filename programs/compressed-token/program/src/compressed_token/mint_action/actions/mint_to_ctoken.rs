use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::Pubkey;
use light_ctoken_interface::{
    instructions::mint_action::ZMintToCTokenAction, state::CompressedMint,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

use crate::{
    compressed_token::mint_action::{accounts::MintActionAccounts, check_authority},
    compressed_token::transfer2::compression::{compress_or_decompress_ctokens, CTokenCompressionInputs},
};

#[allow(clippy::too_many_arguments)]
#[profile]
pub fn process_mint_to_ctoken_action(
    action: &ZMintToCTokenAction,
    compressed_mint: &mut CompressedMint,
    validated_accounts: &MintActionAccounts,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    mint: Pubkey,
    transfer_amount: &mut u64,
    lamports_budget: &mut u64,
) -> Result<(), ProgramError> {
    check_authority(
        compressed_mint.base.mint_authority,
        validated_accounts.authority.key(),
        "mint authority",
    )?;

    let amount = u64::from(action.amount);
    compressed_mint.base.supply = compressed_mint
        .base
        .supply
        .checked_add(amount)
        .ok_or(ErrorCode::MintActionAmountTooLarge)?;

    // Get the recipient token account from packed accounts using the index
    let token_account_info =
        packed_accounts.get_u8(action.account_index, "ctoken mint to recipient")?;

    // Authority check now performed above - safe to proceed with decompression
    // Use the mint_ctokens constructor for simple decompression operations
    let inputs = CTokenCompressionInputs::mint_ctokens(
        amount,
        mint.to_bytes(),
        token_account_info,
        packed_accounts,
    );

    compress_or_decompress_ctokens(inputs, transfer_amount, lamports_budget)
}
