use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::{DataHasher, Hasher};
use light_sdk::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiAccountsConfig, CpiInputs, CpiSigner},
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    LightDiscriminator,
};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

/// Helper function to compress a PDA and reclaim rent.
///
/// 1. closes onchain PDA
/// 2. transfers PDA lamports to rent_recipient
/// 3. updates the empty compressed PDA with onchain PDA data
///
/// This requires the compressed PDA that is tied to the onchain PDA to already
/// exist.
///
/// # Arguments
/// * `pda_account` - The PDA account to compress (will be closed)
/// * `compressed_account_meta` - Metadata for the compressed account (must be
///   empty but have an address)
/// * `proof` - Optional validity proof
/// * `cpi_accounts` - Accounts needed for CPI starting from
///   system_accounts_offset
/// * `system_accounts_offset` - Offset where CPI accounts start
/// * `fee_payer` - The fee payer account
/// * `cpi_signer` - The CPI signer for the calling program
/// * `owner_program` - The program that will own the compressed account
/// * `rent_recipient` - The account to receive the PDA's rent
//
// TODO:
// - rent recipient check, eg hardcoded in caller program
// - check if any explicit checks required for compressed account?
// - check that the account is owned by the owner program, and derived from the correct seeds.
// - consider adding check here that the cAccount belongs to Account via seeds.
pub fn compress_pda<'a, A>(
    pda_account: &AccountInfo<'a>,
    compressed_account_meta: &CompressedAccountMeta,
    proof: Option<ValidityProof>,
    cpi_accounts: &'a [AccountInfo<'a>],
    system_accounts_offset: u8,
    fee_payer: &AccountInfo<'a>,
    cpi_signer: CpiSigner,
    owner_program: &Pubkey,
    rent_recipient: &AccountInfo<'a>,
) -> Result<(), LightSdkError>
where
    A: DataHasher + LightDiscriminator + BorshSerialize + BorshDeserialize + Default,
{
    // Get the PDA lamports before we close it
    let pda_lamports = pda_account.lamports();

    // Always use default/empty data since we're updating an existing compressed account
    let compressed_account =
        LightAccount::<'_, A>::new_mut(owner_program, compressed_account_meta, A::default())?;

    // Set up CPI configuration
    let config = CpiAccountsConfig::new(cpi_signer);

    // Create CPI accounts structure
    let cpi_accounts_struct = CpiAccounts::new_with_config(
        fee_payer,
        &cpi_accounts[system_accounts_offset as usize..],
        config,
    );

    // Create CPI inputs
    let cpi_inputs = CpiInputs::new(
        proof.unwrap_or_default(),
        vec![compressed_account.to_account_info()?],
    );

    // Invoke light system program to create the compressed account
    cpi_inputs.invoke_light_system_program(cpi_accounts_struct)?;

    // Close the PDA account
    // 1. Transfer all lamports to the rent recipient
    let dest_starting_lamports = rent_recipient.lamports();
    **rent_recipient.try_borrow_mut_lamports()? = dest_starting_lamports
        .checked_add(pda_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    // 2. Decrement source account lamports
    **pda_account.try_borrow_mut_lamports()? = 0;
    // 3. Clear all account data
    pda_account.try_borrow_mut_data()?.fill(0);
    // 4. Assign ownership back to the system program
    pda_account.assign(&solana_program::system_program::ID);

    Ok(())
}
