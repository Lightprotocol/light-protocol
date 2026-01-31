//! Compression instruction processor.

use light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo;
use light_sdk_types::{
    instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress, CpiSigner,
};
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    instruction::ValidityProof,
    interface::LightConfig,
    AnchorDeserialize, AnchorSerialize,
};

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
/// Contains internal vec for collecting CompressedAccountInfo results.
pub struct CompressCtx<'a, 'info> {
    pub program_id: &'a Pubkey,
    pub cpi_accounts: &'a CpiAccounts<'a, 'info>,
    pub remaining_accounts: &'a [AccountInfo<'info>],
    pub rent_sponsor: &'a AccountInfo<'info>,
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
/// Takes &mut CompressCtx and pushes CompressedAccountInfo into ctx.compressed_account_infos.
///
/// The dispatch function is responsible for:
/// 1. Reading the discriminator from the account data
/// 2. Deserializing the account based on discriminator
/// 3. Calling prepare_account_for_compression with the deserialized data
pub type CompressDispatchFn<'info> = fn(
    account_info: &AccountInfo<'info>,
    compressed_account_meta: &CompressedAccountMetaNoLamportsNoAddress,
    index: usize,
    ctx: &mut CompressCtx<'_, 'info>,
) -> std::result::Result<(), ProgramError>;

/// Remaining accounts layout:
/// [0]: fee_payer (Signer, mut)
/// [1]: config (LightConfig PDA)
/// [2]: rent_sponsor (mut)
/// [3]: compression_authority (Signer)
/// [system_accounts_offset..]: Light system accounts for CPI
/// [remaining_accounts.len() - num_pda_accounts..]: PDA accounts to compress
///
/// Runtime processor - handles all the plumbing, delegates dispatch to callback.
///
/// **Takes raw instruction data** and deserializes internally - minimizes macro code.
/// **Uses only remaining_accounts** - no Context struct needed.
pub fn process_compress_pda_accounts_idempotent<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    instruction_data: &[u8],
    dispatch_fn: CompressDispatchFn<'info>,
    cpi_signer: CpiSigner,
    program_id: &Pubkey,
) -> std::result::Result<(), ProgramError> {
    // Deserialize params internally
    let params = CompressAndCloseParams::try_from_slice(instruction_data).map_err(|e| {
        solana_msg::msg!("compress: params deser failed: {:?}", e);
        ProgramError::InvalidInstructionData
    })?;

    // Extract and validate accounts using shared validation
    let validated_ctx =
        crate::interface::validation::validate_compress_accounts(remaining_accounts, program_id)?;
    let fee_payer = &validated_ctx.fee_payer;
    let rent_sponsor = &validated_ctx.rent_sponsor;
    let light_config = validated_ctx.light_config;

    let (_, system_accounts) = crate::interface::validation::split_at_system_accounts_offset(
        remaining_accounts,
        params.system_accounts_offset,
    )?;

    let cpi_accounts = CpiAccounts::new(fee_payer, system_accounts, cpi_signer);

    // Build context struct with all needed data (includes internal vec)
    let mut compress_ctx = CompressCtx {
        program_id,
        cpi_accounts: &cpi_accounts,
        remaining_accounts,
        rent_sponsor,
        light_config: &light_config,
        compressed_account_infos: Vec::with_capacity(params.compressed_accounts.len()),
        pda_indices_to_close: Vec::with_capacity(params.compressed_accounts.len()),
        has_non_compressible: false,
    };

    // PDA accounts at end of remaining_accounts
    let pda_accounts = crate::interface::validation::extract_tail_accounts(
        remaining_accounts,
        params.compressed_accounts.len(),
    )?;

    for (i, account_data) in params.compressed_accounts.iter().enumerate() {
        let pda_account = &pda_accounts[i];

        // Skip empty accounts or accounts not owned by this program
        if crate::interface::validation::should_skip_compression(pda_account, program_id) {
            continue;
        }

        // Delegate to dispatch callback (macro-generated match)
        dispatch_fn(pda_account, account_data, i, &mut compress_ctx)?;
    }

    // If any account is not yet compressible, skip the entire batch.
    // The proof covers all accounts so we cannot partially compress.
    if compress_ctx.has_non_compressible {
        return Ok(());
    }

    // CPI to Light System Program
    if !compress_ctx.compressed_account_infos.is_empty() {
        LightSystemProgramCpi::new_cpi(cpi_signer, params.proof)
            .with_account_infos(&compress_ctx.compressed_account_infos)
            .invoke(cpi_accounts.clone())
            .map_err(|e| {
                solana_msg::msg!("compress: CPI failed: {:?}", e);
                ProgramError::Custom(200)
            })?;

        // Close the PDA accounts
        for idx in compress_ctx.pda_indices_to_close {
            let mut info = pda_accounts[idx].clone();
            crate::interface::close::close(&mut info, rent_sponsor).map_err(ProgramError::from)?;
        }
    }

    Ok(())
}
