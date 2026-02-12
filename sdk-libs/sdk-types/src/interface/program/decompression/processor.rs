//! Decompression instruction processor.

#[cfg(feature = "token")]
use alloc::vec;
use alloc::vec::Vec;

use light_account_checks::AccountInfoTrait;
#[cfg(feature = "token")]
use light_account_checks::CpiMeta;
#[cfg(feature = "token")]
use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof,
    with_account_info::{CompressedAccountInfo, InstructionDataInvokeCpiWithAccountInfo},
};
#[cfg(feature = "token")]
use light_token_interface::{
    instructions::{
        extensions::ExtensionInstructionData,
        transfer2::{
            CompressedTokenInstructionDataTransfer2, Compression, MultiInputTokenDataWithContext,
        },
    },
    CPI_AUTHORITY, LIGHT_TOKEN_PROGRAM_ID, TRANSFER2,
};

#[cfg(feature = "token")]
use crate::{
    constants::{
        ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, LIGHT_SYSTEM_PROGRAM_ID,
        REGISTERED_PROGRAM_PDA,
    },
    cpi_accounts::CpiAccountsConfig,
    cpi_context_write::CpiContextWriteAccounts,
};
use crate::{
    cpi_accounts::v2::CpiAccounts,
    error::LightSdkTypesError,
    instruction::PackedStateTreeInfo,
    interface::{
        account::compression_info::CompressedAccountData, cpi::InvokeLightSystemProgram,
        program::config::LightConfig,
    },
    AnchorDeserialize, AnchorSerialize, CpiSigner,
};

/// Account indices within remaining_accounts for decompress instructions.
const FEE_PAYER_INDEX: usize = 0;
const CONFIG_INDEX: usize = 1;
const RENT_SPONSOR_INDEX: usize = 2;

// ============================================================================
// DecompressVariant Trait
// ============================================================================

/// Trait for packed program account variants that support decompression.
///
/// Implemented by the program's `PackedProgramAccountVariant` enum
/// to handle type-specific dispatch during decompression.
///
/// MACRO-GENERATED: The implementation contains a match statement routing each
/// enum variant to the appropriate `prepare_account_for_decompression` call.
pub trait DecompressVariant<AI: AccountInfoTrait + Clone>:
    AnchorSerialize + AnchorDeserialize + Clone
{
    /// Decompress this variant into a PDA account.
    ///
    /// The implementation should match on the enum variant and call
    /// `prepare_account_for_decompression::<SEED_COUNT, PackedVariantType>(packed, pda_account, ctx)`.
    fn decompress(
        &self,
        meta: &PackedStateTreeInfo,
        pda_account: &AI,
        ctx: &mut DecompressCtx<'_, AI>,
    ) -> Result<(), LightSdkTypesError>;
}

// ============================================================================
// Parameters and Context
// ============================================================================

/// Parameters for decompress_idempotent instruction.
/// Generic over the variant type - each program defines its own `PackedProgramAccountVariant`.
///
/// Field order matches `LoadAccountsData` from light-client for compatibility.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DecompressIdempotentParams<V>
where
    V: AnchorSerialize + AnchorDeserialize + Clone,
{
    /// Offset into remaining_accounts where Light system accounts begin
    pub system_accounts_offset: u8,
    /// Accounts before this offset are PDA accounts, at and after are token accounts.
    /// Set to accounts.len() if no token accounts.
    pub token_accounts_offset: u8,
    /// Packed index of the output queue in remaining_accounts.
    pub output_queue_index: u8,
    /// Validity proof for compressed account verification
    pub proof: ValidityProof,
    /// Accounts to decompress - wrapped in CompressedAccountData for metadata
    pub accounts: Vec<CompressedAccountData<V>>,
}

/// Context struct holding all data needed for decompression.
/// Generic over AccountInfoTrait to work with both solana and pinocchio.
pub struct DecompressCtx<'a, AI: AccountInfoTrait + Clone> {
    pub program_id: &'a [u8; 32],
    pub cpi_accounts: &'a CpiAccounts<'a, AI>,
    pub remaining_accounts: &'a [AI],
    pub rent_sponsor: &'a AI,
    /// Rent sponsor PDA bump for signing
    pub rent_sponsor_bump: u8,
    pub light_config: &'a LightConfig,
    pub current_slot: u64,
    /// Packed index of the output queue in remaining_accounts.
    pub output_queue_index: u8,
    /// Internal vec - dispatch functions push results here
    pub compressed_account_infos: Vec<CompressedAccountInfo>,
    // Token-specific fields (only present when token feature is enabled)
    #[cfg(feature = "token")]
    pub ctoken_rent_sponsor: Option<&'a AI>,
    #[cfg(feature = "token")]
    pub ctoken_compressible_config: Option<&'a AI>,
    #[cfg(feature = "token")]
    pub in_token_data: Vec<MultiInputTokenDataWithContext>,
    #[cfg(feature = "token")]
    pub in_tlv: Option<Vec<Vec<ExtensionInstructionData>>>,
    #[cfg(feature = "token")]
    pub token_seeds: Vec<Vec<u8>>,
}

// ============================================================================
// PDA-only Processor
// ============================================================================

/// Process decompression for PDA accounts (idempotent, PDA-only).
///
/// Iterates over PDA accounts, dispatches each for decompression via `DecompressVariant`,
/// then invokes the Light system program CPI to commit compressed state.
///
/// Idempotent: if a PDA is already initialized, it is silently skipped.
///
/// # Account layout in remaining_accounts:
/// - `[0]`: fee_payer (Signer, mut)
/// - `[1]`: config (LightConfig PDA)
/// - `[2]`: rent_sponsor (mut)
/// - `[system_accounts_offset..hot_accounts_start]`: Light system + tree accounts
/// - `[hot_accounts_start..]`: PDA accounts to decompress into
#[inline(never)]
pub fn process_decompress_pda_accounts_idempotent<AI, V>(
    remaining_accounts: &[AI],
    params: &DecompressIdempotentParams<V>,
    cpi_signer: CpiSigner,
    program_id: &[u8; 32],
    current_slot: u64,
) -> Result<(), LightSdkTypesError>
where
    AI: AccountInfoTrait + Clone,
    V: DecompressVariant<AI>,
{
    let system_accounts_offset = params.system_accounts_offset as usize;
    if system_accounts_offset > remaining_accounts.len() {
        return Err(LightSdkTypesError::InvalidInstructionData);
    }

    // PDA accounts: all accounts up to token_accounts_offset
    let num_pda_accounts = params.token_accounts_offset as usize;
    let pda_accounts = params
        .accounts
        .get(..num_pda_accounts)
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;

    if pda_accounts.is_empty() {
        return Err(LightSdkTypesError::InvalidInstructionData);
    }

    // 2. Load and validate config
    let config = LightConfig::load_checked(&remaining_accounts[CONFIG_INDEX], program_id)?;
    let rent_sponsor = &remaining_accounts[RENT_SPONSOR_INDEX];
    let rent_sponsor_bump = config.validate_rent_sponsor_account::<AI>(rent_sponsor)?;

    // 3. Hot accounts (PDAs) at the tail of remaining_accounts
    let num_hot_accounts = params.accounts.len();
    let hot_accounts_start = remaining_accounts
        .len()
        .checked_sub(num_hot_accounts)
        .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?;
    let hot_account_infos = &remaining_accounts[hot_accounts_start..];
    let pda_account_infos = hot_account_infos
        .get(..num_pda_accounts)
        .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?;

    // 4. Build CpiAccounts (system + tree accounts, excluding hot accounts)
    let cpi_accounts = CpiAccounts::new(
        &remaining_accounts[FEE_PAYER_INDEX],
        &remaining_accounts[system_accounts_offset..hot_accounts_start],
        cpi_signer,
    );

    // 5. Build context and dispatch (scoped to release borrows before CPI)
    let compressed_account_infos = {
        let mut decompress_ctx = DecompressCtx {
            program_id,
            cpi_accounts: &cpi_accounts,
            remaining_accounts,
            rent_sponsor,
            rent_sponsor_bump,
            light_config: &config,
            current_slot,
            output_queue_index: params.output_queue_index,
            compressed_account_infos: Vec::with_capacity(num_pda_accounts),
            #[cfg(feature = "token")]
            ctoken_rent_sponsor: None,
            #[cfg(feature = "token")]
            ctoken_compressible_config: None,
            #[cfg(feature = "token")]
            in_token_data: Vec::new(),
            #[cfg(feature = "token")]
            in_tlv: None,
            #[cfg(feature = "token")]
            token_seeds: Vec::new(),
        };

        for (pda_account_data, pda_account_info) in pda_accounts.iter().zip(pda_account_infos) {
            pda_account_data.data.decompress(
                &pda_account_data.tree_info,
                pda_account_info,
                &mut decompress_ctx,
            )?;
        }

        decompress_ctx.compressed_account_infos
    };

    // 6. If no compressed accounts were produced (all already initialized), skip CPI
    if compressed_account_infos.is_empty() {
        return Ok(());
    }

    // 7. Build and invoke Light system program CPI
    let mut cpi_ix_data = InstructionDataInvokeCpiWithAccountInfo::new(
        program_id.into(),
        cpi_signer.bump,
        params.proof.into(),
    );
    cpi_ix_data.account_infos = compressed_account_infos;
    cpi_ix_data.invoke::<AI>(cpi_accounts)?;

    Ok(())
}

// ============================================================================
// Full Processor (PDA + Token)
// ============================================================================

/// Process decompression for both PDA and token accounts (idempotent).
///
/// Handles the combined PDA + token decompression flow:
/// - PDA accounts are decompressed first
/// - If both PDAs and tokens exist, PDA data is written to CPI context first
/// - Token accounts are decompressed via Transfer2 CPI to the light token program
///
/// # Account layout in remaining_accounts:
/// - `[0]`: fee_payer (Signer, mut)
/// - `[1]`: config (LightConfig PDA)
/// - `[2]`: rent_sponsor (mut)
/// - `[3]`: ctoken_rent_sponsor (mut)
/// - `[4]`: light_token_program
/// - `[5]`: cpi_authority
/// - `[6]`: ctoken_compressible_config
/// - `[system_accounts_offset..hot_accounts_start]`: Light system + tree accounts
/// - `[hot_accounts_start..]`: Hot accounts (PDAs then tokens)
#[cfg(feature = "token")]
#[inline(never)]
pub fn process_decompress_accounts_idempotent<AI, V>(
    remaining_accounts: &[AI],
    params: &DecompressIdempotentParams<V>,
    cpi_signer: CpiSigner,
    program_id: &[u8; 32],
    current_slot: u64,
) -> Result<(), LightSdkTypesError>
where
    AI: AccountInfoTrait + Clone,
    V: DecompressVariant<AI>,
{
    let system_accounts_offset = params.system_accounts_offset as usize;
    if system_accounts_offset > remaining_accounts.len() {
        return Err(LightSdkTypesError::InvalidInstructionData);
    }

    // 2. Split accounts into PDA and token
    let (pda_accounts, token_accounts) = params
        .accounts
        .split_at_checked(params.token_accounts_offset as usize)
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;

    // 3. Load and validate config
    let config = LightConfig::load_checked(&remaining_accounts[CONFIG_INDEX], program_id)?;
    let rent_sponsor = &remaining_accounts[RENT_SPONSOR_INDEX];
    let rent_sponsor_bump = config.validate_rent_sponsor_account::<AI>(rent_sponsor)?;

    // 4. Hot accounts at the tail of remaining_accounts
    let num_hot_accounts = params.accounts.len();
    let hot_accounts_start = remaining_accounts
        .len()
        .checked_sub(num_hot_accounts)
        .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?;
    let hot_account_infos = &remaining_accounts[hot_accounts_start..];
    let (pda_account_infos, token_account_infos) = hot_account_infos
        .split_at_checked(params.token_accounts_offset as usize)
        .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?;

    let has_pda_accounts = !pda_accounts.is_empty();
    let has_token_accounts = !token_accounts.is_empty();
    let cpi_context = has_pda_accounts && has_token_accounts;

    // 5. Build CpiAccounts
    let cpi_config = CpiAccountsConfig {
        sol_compression_recipient: false,
        sol_pool_pda: false,
        cpi_context,
        cpi_signer,
    };
    let cpi_accounts = CpiAccounts::new_with_config(
        &remaining_accounts[FEE_PAYER_INDEX],
        &remaining_accounts[system_accounts_offset..hot_accounts_start],
        cpi_config,
    );

    // Token (ctoken) accounts layout (only required when token accounts are present):
    // [3] ctoken_rent_sponsor, [6] ctoken_compressible_config
    let (ctoken_rent_sponsor, ctoken_compressible_config) = if has_token_accounts {
        let rent_sponsor = remaining_accounts
            .get(3)
            .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?;
        let config = remaining_accounts
            .get(6)
            .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?;
        (Some(rent_sponsor), Some(config))
    } else {
        (None, None)
    };

    // 6. Build context and dispatch (scoped to release borrows before CPI)
    let (compressed_account_infos, in_token_data, in_tlv, token_seeds) = {
        let mut decompress_ctx = DecompressCtx {
            program_id,
            cpi_accounts: &cpi_accounts,
            remaining_accounts,
            rent_sponsor,
            rent_sponsor_bump,
            light_config: &config,
            current_slot,
            output_queue_index: params.output_queue_index,
            compressed_account_infos: Vec::new(),
            ctoken_rent_sponsor,
            ctoken_compressible_config,
            in_token_data: Vec::new(),
            in_tlv: None,
            token_seeds: Vec::new(),
        };

        // Process PDA accounts
        for (pda_account_data, pda_account_info) in pda_accounts.iter().zip(pda_account_infos) {
            pda_account_data.data.decompress(
                &pda_account_data.tree_info,
                pda_account_info,
                &mut decompress_ctx,
            )?;
        }

        // Process token accounts
        for (token_account_data, token_account_info) in
            token_accounts.iter().zip(token_account_infos)
        {
            token_account_data.data.decompress(
                &token_account_data.tree_info,
                token_account_info,
                &mut decompress_ctx,
            )?;
        }

        (
            decompress_ctx.compressed_account_infos,
            decompress_ctx.in_token_data,
            decompress_ctx.in_tlv,
            decompress_ctx.token_seeds,
        )
    };

    // 7. PDA CPI (Light system program)
    if has_pda_accounts {
        let pda_only = !cpi_context;

        if pda_only {
            let mut cpi_ix_data = InstructionDataInvokeCpiWithAccountInfo::new(
                program_id.into(),
                cpi_signer.bump,
                params.proof.into(),
            );
            cpi_ix_data.account_infos = compressed_account_infos;
            cpi_ix_data.invoke::<AI>(cpi_accounts.clone())?;
        } else {
            // PDAs + tokens: write PDA data to CPI context first, tokens will execute
            let authority = cpi_accounts.authority()?;
            let cpi_context_account = cpi_accounts.cpi_context()?;
            let system_cpi_accounts = CpiContextWriteAccounts {
                fee_payer: &remaining_accounts[FEE_PAYER_INDEX],
                authority,
                cpi_context: cpi_context_account,
                cpi_signer,
            };

            let cpi_ix_data = InstructionDataInvokeCpiWithAccountInfo {
                mode: 1,
                bump: cpi_signer.bump,
                invoking_program_id: cpi_signer.program_id.into(),
                compress_or_decompress_lamports: 0,
                is_compress: false,
                with_cpi_context: true,
                with_transaction_hash: false,
                cpi_context: CompressedCpiContext::first(),
                proof: None,
                new_address_params: Vec::new(),
                account_infos: compressed_account_infos,
                read_only_addresses: Vec::new(),
                read_only_accounts: Vec::new(),
            };
            cpi_ix_data.invoke_write_to_cpi_context_first(system_cpi_accounts)?;
        }
    }

    // 8. Token CPI (Transfer2 to light token program)
    if has_token_accounts {
        let mut compressions = Vec::new();
        for a in &in_token_data {
            compressions.push(Compression::decompress(a.amount, a.mint, a.owner));
        }

        let mut cpi = CompressedTokenInstructionDataTransfer2 {
            with_transaction_hash: false,
            in_token_data: in_token_data.clone(),
            in_tlv: in_tlv.clone(),
            with_lamports_change_account_merkle_tree_index: false,
            lamports_change_account_merkle_tree_index: 0,
            lamports_change_account_owner_index: 0,
            output_queue: 0,
            max_top_up: u16::MAX, // No limit
            cpi_context: None,
            compressions: Some(compressions),
            proof: params.proof.0,
            out_token_data: Vec::new(),
            in_lamports: None,
            out_lamports: None,
            out_tlv: None,
        };

        if has_pda_accounts {
            cpi.cpi_context = Some(
                light_token_interface::instructions::transfer2::CompressedCpiContext {
                    set_context: false,
                    first_set_context: false,
                },
            );
        }

        // Build Transfer2 account metas in the order the handler expects:
        // [0] light_system_program (readonly)
        // [1] fee_payer (signer, writable)
        // [2] cpi_authority_pda (readonly)
        // [3] registered_program_pda (readonly)
        // [4] account_compression_authority (readonly)
        // [5] account_compression_program (readonly)
        // [6] system_program (readonly)
        // [7] cpi_context (optional, writable)
        // [N+] packed_accounts
        let fee_payer_key = remaining_accounts[FEE_PAYER_INDEX].key();
        let mut account_metas = vec![
            CpiMeta {
                pubkey: LIGHT_SYSTEM_PROGRAM_ID,
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: fee_payer_key,
                is_signer: true,
                is_writable: true,
            },
            CpiMeta {
                pubkey: CPI_AUTHORITY,
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: REGISTERED_PROGRAM_PDA,
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: ACCOUNT_COMPRESSION_AUTHORITY_PDA,
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: ACCOUNT_COMPRESSION_PROGRAM_ID,
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: [0u8; 32],
                is_signer: false,
                is_writable: false,
            }, // system_program
        ];

        if cpi_context {
            let cpi_ctx = cpi_accounts.cpi_context()?;
            account_metas.push(CpiMeta {
                pubkey: cpi_ctx.key(),
                is_signer: false,
                is_writable: true,
            });
        }

        let transfer2_packed_start = account_metas.len();
        let packed_accounts_offset =
            system_accounts_offset + cpi_accounts.system_accounts_end_offset();
        for account in &remaining_accounts[packed_accounts_offset..] {
            account_metas.push(CpiMeta {
                pubkey: account.key(),
                is_signer: account.is_signer(),
                is_writable: account.is_writable(),
            });
        }

        // Mark owner accounts as signers for the Transfer2 CPI
        for data in &in_token_data {
            account_metas[data.owner as usize + transfer2_packed_start].is_signer = true;
        }

        // Serialize instruction data
        let mut transfer2_data = vec![TRANSFER2];
        cpi.serialize(&mut transfer2_data)
            .map_err(|_| LightSdkTypesError::Borsh)?;

        // Invoke the light token program
        if token_seeds.is_empty() {
            // All ATAs - no PDA signing needed
            AI::invoke_cpi(
                &LIGHT_TOKEN_PROGRAM_ID,
                &transfer2_data,
                &account_metas,
                remaining_accounts,
                &[],
            )
            .map_err(|e| LightSdkTypesError::ProgramError(e.into()))?;
        } else {
            // At least one regular token account - use invoke_signed with PDA seeds
            let signer_seed_refs: Vec<&[u8]> = token_seeds.iter().map(|s| s.as_slice()).collect();
            AI::invoke_cpi(
                &LIGHT_TOKEN_PROGRAM_ID,
                &transfer2_data,
                &account_metas,
                remaining_accounts,
                &[signer_seed_refs.as_slice()],
            )
            .map_err(|e| LightSdkTypesError::ProgramError(e.into()))?;
        }
    }

    Ok(())
}
