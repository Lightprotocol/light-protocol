//! Compression instruction processor.

use light_account_checks::AccountInfoTrait;
use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof,
    with_account_info::{CompressedAccountInfo, InstructionDataInvokeCpiWithAccountInfo},
};
use light_sdk_types::{
    cpi_accounts::v2::CpiAccounts, instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
    CpiSigner,
};

use crate::{
    cpi::InvokeLightSystemProgram,
    error::LightPdaError,
    program::{compression::close::close, config::LightConfig},
    AnchorDeserialize, AnchorSerialize,
};

/// Account indices within remaining_accounts for compress instructions.
const FEE_PAYER_INDEX: usize = 0;
const CONFIG_INDEX: usize = 1;
const RENT_SPONSOR_INDEX: usize = 2;

/// Parameters for compress_and_close instruction.
/// Matches SDK's SaveAccountsData field order for compatibility.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CompressAndCloseParams {
    /// Validity proof for compressed account verification
    pub proof: ValidityProof,
    /// Accounts to compress (meta only - data read from PDA)
    pub compressed_accounts: Vec<CompressedAccountMetaNoLamportsNoAddress>,
    /// Offset into remaining_accounts where Light system accounts begin
    pub system_accounts_offset: u8,
}

/// Context struct holding all data needed for compression.
/// Generic over AccountInfoTrait to work with both solana and pinocchio.
pub struct CompressCtx<'a, AI: AccountInfoTrait> {
    pub program_id: &'a [u8; 32],
    pub remaining_accounts: &'a [AI],
    pub rent_sponsor: &'a AI,
    pub light_config: &'a LightConfig,
    /// Internal vec - dispatch functions push results here
    pub compressed_account_infos: Vec<CompressedAccountInfo>,
    /// Track which PDA indices to close
    pub pda_indices_to_close: Vec<usize>,
    /// Set to true if any account is not yet compressible.
    /// When set, the entire batch is skipped (no CPI, no closes).
    pub has_non_compressible: bool,
}

/// Callback type for discriminator-based dispatch.
/// MACRO-GENERATED: Just a match statement routing to prepare_account_for_compression.
pub type CompressDispatchFn<AI> = fn(
    account_info: &AI,
    compressed_account_meta: &CompressedAccountMetaNoLamportsNoAddress,
    index: usize,
    ctx: &mut CompressCtx<'_, AI>,
) -> Result<(), LightPdaError>;

/// Process compress-and-close for PDA accounts (idempotent).
///
/// Iterates over PDA accounts, dispatches each for compression via `dispatch_fn`,
/// then invokes the Light system program CPI to commit compressed state,
/// and closes the PDA accounts (transferring lamports to rent_sponsor).
///
/// Idempotent: if any account is not yet compressible (rent function check fails),
/// the entire batch is silently skipped.
#[inline(never)]
pub fn process_compress_pda_accounts_idempotent<AI: AccountInfoTrait + Clone>(
    remaining_accounts: &[AI],
    instruction_data: &[u8],
    dispatch_fn: CompressDispatchFn<AI>,
    cpi_signer: CpiSigner,
    program_id: &[u8; 32],
) -> Result<(), LightPdaError> {
    // 1. Deserialize params
    let params = CompressAndCloseParams::try_from_slice(instruction_data)
        .map_err(|_| LightPdaError::Borsh)?;

    let system_accounts_offset = params.system_accounts_offset as usize;
    let num_pdas = params.compressed_accounts.len();

    if num_pdas == 0 {
        return Err(LightPdaError::InvalidInstructionData);
    }

    // 2. Load and validate config
    let config = LightConfig::load_checked(&remaining_accounts[CONFIG_INDEX], program_id)?;

    // 3. Validate rent_sponsor
    let rent_sponsor = &remaining_accounts[RENT_SPONSOR_INDEX];
    config.validate_rent_sponsor_account::<AI>(rent_sponsor)?;

    // 4. PDA accounts are at the tail of remaining_accounts
    let pda_start = remaining_accounts
        .len()
        .checked_sub(num_pdas)
        .ok_or(LightPdaError::NotEnoughAccountKeys)?;

    // 5. Run dispatch for each PDA
    let (compressed_account_infos, pda_indices_to_close, has_non_compressible) = {
        let mut ctx = CompressCtx {
            program_id,
            remaining_accounts,
            rent_sponsor,
            light_config: &config,
            compressed_account_infos: Vec::with_capacity(num_pdas),
            pda_indices_to_close: Vec::with_capacity(num_pdas),
            has_non_compressible: false,
        };

        for (i, meta) in params.compressed_accounts.iter().enumerate() {
            let pda_index = pda_start + i;
            dispatch_fn(&remaining_accounts[pda_index], meta, pda_index, &mut ctx)?;
        }

        (
            ctx.compressed_account_infos,
            ctx.pda_indices_to_close,
            ctx.has_non_compressible,
        )
    };

    // 6. Idempotent: if any account is not yet compressible, skip entire batch
    if has_non_compressible {
        return Ok(());
    }

    // 7. Build CPI instruction data
    let mut cpi_ix_data = InstructionDataInvokeCpiWithAccountInfo::new(
        program_id.into(),
        cpi_signer.bump,
        params.proof.into(),
    );
    cpi_ix_data.account_infos = compressed_account_infos;

    // 8. Build CpiAccounts from system accounts slice (excluding PDA accounts at tail)
    let cpi_accounts = CpiAccounts::new(
        &remaining_accounts[FEE_PAYER_INDEX],
        &remaining_accounts[system_accounts_offset..pda_start],
        cpi_signer,
    );

    // 9. Invoke Light system program CPI
    cpi_ix_data.invoke::<AI>(cpi_accounts)?;

    // 10. Close PDA accounts, transferring lamports to rent_sponsor
    for pda_index in &pda_indices_to_close {
        close(&remaining_accounts[*pda_index], rent_sponsor)?;
    }

    Ok(())
}
