//! Reusable `create_accounts` function for creating PDAs, mints, token vaults,
//! and ATAs in a single instruction. Used by both `#[derive(LightAccounts)]`
//! macro-generated code and manual implementations.

use alloc::{vec, vec::Vec};

use light_account_checks::AccountInfoTrait;
use light_compressed_account::{
    instruction_data::{
        cpi_context::CompressedCpiContext,
        with_account_info::InstructionDataInvokeCpiWithAccountInfo,
    },
    CpiSigner,
};

use crate::{
    cpi_accounts::{v2::CpiAccounts, CpiAccountsConfig},
    cpi_context_write::CpiContextWriteAccounts,
    error::LightSdkTypesError,
    interface::{
        accounts::init_compressed_account::prepare_compressed_account_on_init,
        cpi::{
            create_mints::{CreateMints, CreateMintsStaticAccounts, SingleMintParams},
            create_token_accounts::{CreateTokenAccountCpi, CreateTokenAtaCpi},
            invoke::InvokeLightSystemProgram,
        },
        create_accounts_proof::CreateAccountsProof,
        program::config::LightConfig,
    },
};

// ============================================================================
// Parameter structs
// ============================================================================

/// Parameters for a single PDA to initialize.
pub struct PdaInitParam<'a, AI: AccountInfoTrait> {
    /// The PDA account to register as a compressed account.
    pub account: &'a AI,
}

/// Input for creating compressed mints.
///
/// Uses owned arrays because `CreateMints` expects `&[AI]` slices,
/// and Anchor requires `.to_account_info()` conversion.
pub struct CreateMintsInput<'a, AI: AccountInfoTrait + Clone, const MINTS: usize> {
    /// Per-mint parameters (decimals, authority, seeds, etc.).
    pub params: [SingleMintParams<'a>; MINTS],
    /// Mint seed accounts (signers) - one per mint.
    pub mint_seed_accounts: [AI; MINTS],
    /// Mint PDA accounts (writable) - one per mint.
    pub mint_accounts: [AI; MINTS],
}

/// Parameters for a single token vault to create.
pub struct TokenInitParam<'a, AI: AccountInfoTrait> {
    /// The token vault account.
    pub account: &'a AI,
    /// The mint account for this vault.
    pub mint: &'a AI,
    /// Owner of the token account (as raw bytes).
    pub owner: [u8; 32],
    /// PDA seeds for the vault (with bump as last element).
    pub seeds: &'a [&'a [u8]],
}

/// Parameters for a single ATA to create.
pub struct AtaInitParam<'a, AI: AccountInfoTrait> {
    /// The ATA account.
    pub ata: &'a AI,
    /// The owner of the ATA.
    pub owner: &'a AI,
    /// The mint for the ATA.
    pub mint: &'a AI,
    /// Whether to use idempotent creation.
    pub idempotent: bool,
}

/// Shared accounts needed across all account creation operations.
pub struct SharedAccounts<'a, AI: AccountInfoTrait> {
    /// Fee payer for the transaction.
    pub fee_payer: &'a AI,
    /// CPI signer for the program.
    pub cpi_signer: CpiSigner,
    /// Proof data containing tree indices, validity proof, etc.
    pub proof: &'a CreateAccountsProof,
    /// Program ID (as raw bytes).
    pub program_id: [u8; 32],
    /// Compression config account. Required if PDAS > 0.
    pub compression_config: Option<&'a AI>,
    /// Compressible config account. Required if MINTS > 0 or TOKENS > 0 or ATAS > 0.
    pub compressible_config: Option<&'a AI>,
    /// Rent sponsor account. Required if MINTS > 0 or TOKENS > 0 or ATAS > 0.
    pub rent_sponsor: Option<&'a AI>,
    /// CPI authority account. Required if MINTS > 0.
    pub cpi_authority: Option<&'a AI>,
    /// System program account. Required if TOKENS > 0 or ATAS > 0.
    pub system_program: Option<&'a AI>,
}

// ============================================================================
// Main function
// ============================================================================

/// Create compressed PDAs, mints, token vaults, and ATAs in a single instruction.
///
/// Returns `true` if CPI context was set up (MINTS > 0), `false` otherwise.
///
/// # Const Generics
///
/// - `PDAS`: Number of compressed PDAs to register.
/// - `MINTS`: Number of compressed mints to create via `CreateMints`.
/// - `TOKENS`: Number of PDA token vaults to create via `CreateTokenAccountCpi`.
/// - `ATAS`: Number of ATAs to create via `CreateTokenAtaCpi`.
///
/// # Type Parameters
///
/// - `AI`: Account info type (`AccountInfoTrait`).
/// - `F`: Closure called after all PDAs are prepared, before CPI context write.
///   Signature: `FnOnce(&LightConfig, u64) -> Result<(), LightSdkTypesError>`.
///   The closure receives the loaded `LightConfig` and current slot.
///   When `PDAS = 0`, pass `|_, _| Ok(())`.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn create_accounts<
    AI: AccountInfoTrait + Clone,
    const PDAS: usize,
    const MINTS: usize,
    const TOKENS: usize,
    const ATAS: usize,
    F: FnOnce(&LightConfig, u64) -> Result<(), LightSdkTypesError>,
>(
    pdas: [PdaInitParam<'_, AI>; PDAS],
    pda_setup: F,
    mints: Option<CreateMintsInput<'_, AI, MINTS>>,
    tokens: [TokenInitParam<'_, AI>; TOKENS],
    atas: [AtaInitParam<'_, AI>; ATAS],
    shared: &SharedAccounts<'_, AI>,
    remaining_accounts: &[AI],
) -> Result<bool, LightSdkTypesError> {
    // ====================================================================
    // 1. Validate required Option fields based on const generics
    // ====================================================================
    if PDAS > u8::MAX as usize
        || MINTS > u8::MAX as usize
        || PDAS.saturating_add(MINTS) > u8::MAX as usize
    {
        return Err(LightSdkTypesError::InvalidInstructionData);
    }
    if PDAS > 0 && shared.compression_config.is_none() {
        return Err(LightSdkTypesError::InvalidInstructionData);
    }
    let has_tokens = MINTS > 0 || TOKENS > 0 || ATAS > 0;
    if has_tokens && (shared.compressible_config.is_none() || shared.rent_sponsor.is_none()) {
        return Err(LightSdkTypesError::InvalidInstructionData);
    }
    if MINTS > 0 && shared.cpi_authority.is_none() {
        return Err(LightSdkTypesError::InvalidInstructionData);
    }
    if (TOKENS > 0 || ATAS > 0) && shared.system_program.is_none() {
        return Err(LightSdkTypesError::InvalidInstructionData);
    }

    // CPI context is needed whenever mints are created:
    // - The client always packs a CPI context account for mint creation
    // - Single mint (N=1): CPI context account is present but invoke_single_mint skips it
    // - Multi-mint (N>1): CPI context used for batching (N-1 writes + 1 execute)
    // - PDAs + mints: PDAs write to CPI context first, then mints execute with offset
    let with_cpi_context = MINTS > 0;

    // ====================================================================
    // 2. Build CPI accounts
    // ====================================================================
    let cpi_accounts = if PDAS > 0 || MINTS > 0 {
        let system_accounts_offset = shared.proof.system_accounts_offset as usize;
        if remaining_accounts.len() < system_accounts_offset {
            return Err(LightSdkTypesError::FewerAccountsThanSystemAccounts);
        }
        let config = if with_cpi_context {
            CpiAccountsConfig::new_with_cpi_context(shared.cpi_signer)
        } else {
            CpiAccountsConfig::new(shared.cpi_signer)
        };
        Some(CpiAccounts::new_with_config(
            shared.fee_payer,
            &remaining_accounts[system_accounts_offset..],
            config,
        ))
    } else {
        None
    };

    // ====================================================================
    // 3. Create PDAs
    // ====================================================================
    // pda_setup is intentionally not called when PDAS == 0; callers should
    // pass `|_, _| Ok(())` in that case (see function doc).
    if PDAS > 0 {
        create_pdas(
            &pdas,
            pda_setup,
            shared,
            cpi_accounts
                .as_ref()
                .expect("cpi_accounts is built when PDAS > 0"),
            with_cpi_context,
        )?;
    } else {
        drop(pda_setup);
    }

    // ====================================================================
    // 4. Create Mints
    // ====================================================================
    if MINTS > 0 {
        if let Some(mints_input) = mints {
            create_mints_inner::<AI, MINTS>(
                mints_input,
                shared,
                cpi_accounts
                    .as_ref()
                    .expect("cpi_accounts is built when MINTS > 0"),
                PDAS as u8,
            )?;
        } else {
            return Err(LightSdkTypesError::InvalidInstructionData);
        }
    }

    // ====================================================================
    // 5. Create Token Vaults
    // ====================================================================
    if TOKENS > 0 {
        create_token_vaults(&tokens, shared)?;
    }

    // ====================================================================
    // 6. Create ATAs
    // ====================================================================
    if ATAS > 0 {
        create_atas(&atas, shared)?;
    }

    Ok(with_cpi_context)
}

// ============================================================================
// Internal helpers
// ============================================================================

#[inline(never)]
fn create_pdas<AI: AccountInfoTrait + Clone, F>(
    pdas: &[PdaInitParam<'_, AI>],
    pda_setup: F,
    shared: &SharedAccounts<'_, AI>,
    cpi_accounts: &CpiAccounts<'_, AI>,
    with_cpi_context: bool,
) -> Result<(), LightSdkTypesError>
where
    F: FnOnce(&LightConfig, u64) -> Result<(), LightSdkTypesError>,
{
    let address_tree_info = &shared.proof.address_tree_info;
    let address_tree_account = cpi_accounts
        .get_tree_account_info(address_tree_info.address_merkle_tree_pubkey_index as usize)?;
    let address_tree_pubkey = address_tree_account.key();
    let output_tree_index = shared.proof.output_state_tree_index;

    // Load config and get current slot
    let compression_config = shared
        .compression_config
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;
    let light_config = LightConfig::load_checked(compression_config, &shared.program_id)
        .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;

    let current_slot =
        AI::get_current_slot().map_err(|_| LightSdkTypesError::InvalidInstructionData)?;

    // Prepare all PDAs
    let cpi_context = if with_cpi_context {
        CompressedCpiContext::first()
    } else {
        CompressedCpiContext::default()
    };

    let mut new_address_params = Vec::with_capacity(pdas.len());
    let mut account_infos = Vec::with_capacity(pdas.len());

    for (i, pda) in pdas.iter().enumerate() {
        let pda_key = pda.account.key();
        prepare_compressed_account_on_init(
            &pda_key,
            &address_tree_pubkey,
            address_tree_info,
            output_tree_index,
            i as u8,
            &shared.program_id,
            &mut new_address_params,
            &mut account_infos,
        )?;
    }

    // Call the user's setup closure (e.g., set_decompressed on each PDA)
    pda_setup(&light_config, current_slot)?;

    // Build instruction data
    let instruction_data = InstructionDataInvokeCpiWithAccountInfo {
        mode: 1, // V2 mode
        bump: shared.cpi_signer.bump,
        invoking_program_id: shared.cpi_signer.program_id.into(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context,
        with_transaction_hash: false,
        cpi_context,
        proof: shared.proof.proof.0,
        new_address_params,
        account_infos,
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    if with_cpi_context {
        // Write to CPI context first (combined execution happens with mints)
        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: cpi_accounts.fee_payer(),
            authority: cpi_accounts.authority()?,
            cpi_context: cpi_accounts.cpi_context()?,
            cpi_signer: shared.cpi_signer,
        };
        instruction_data.invoke_write_to_cpi_context_first(cpi_context_accounts)?;
    } else {
        // Direct invocation (no mints following)
        instruction_data.invoke(cpi_accounts.clone())?;
    }

    Ok(())
}

#[inline(never)]
fn create_mints_inner<AI: AccountInfoTrait + Clone, const MINTS: usize>(
    mints_input: CreateMintsInput<'_, AI, MINTS>,
    shared: &SharedAccounts<'_, AI>,
    cpi_accounts: &CpiAccounts<'_, AI>,
    cpi_context_offset: u8,
) -> Result<(), LightSdkTypesError> {
    let compressible_config = shared
        .compressible_config
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;
    let rent_sponsor = shared
        .rent_sponsor
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;
    let cpi_authority = shared
        .cpi_authority
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;

    CreateMints {
        mints: &mints_input.params,
        proof_data: shared.proof,
        mint_seed_accounts: &mints_input.mint_seed_accounts,
        mint_accounts: &mints_input.mint_accounts,
        static_accounts: CreateMintsStaticAccounts {
            fee_payer: shared.fee_payer,
            compressible_config,
            rent_sponsor,
            cpi_authority,
        },
        cpi_context_offset,
    }
    .invoke(cpi_accounts)
}

#[inline(never)]
fn create_token_vaults<AI: AccountInfoTrait + Clone>(
    tokens: &[TokenInitParam<'_, AI>],
    shared: &SharedAccounts<'_, AI>,
) -> Result<(), LightSdkTypesError> {
    let compressible_config = shared
        .compressible_config
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;
    let rent_sponsor = shared
        .rent_sponsor
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;
    let system_program = shared
        .system_program
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;

    for token in tokens {
        CreateTokenAccountCpi {
            payer: shared.fee_payer,
            account: token.account,
            mint: token.mint,
            owner: token.owner,
        }
        .rent_free(
            compressible_config,
            rent_sponsor,
            system_program,
            &shared.program_id,
        )
        .invoke_signed(token.seeds)?;
    }

    Ok(())
}

#[inline(never)]
fn create_atas<AI: AccountInfoTrait + Clone>(
    atas: &[AtaInitParam<'_, AI>],
    shared: &SharedAccounts<'_, AI>,
) -> Result<(), LightSdkTypesError> {
    let compressible_config = shared
        .compressible_config
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;
    let rent_sponsor = shared
        .rent_sponsor
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;
    let system_program = shared
        .system_program
        .ok_or(LightSdkTypesError::InvalidInstructionData)?;

    for ata in atas {
        let cpi = CreateTokenAtaCpi {
            payer: shared.fee_payer,
            owner: ata.owner,
            mint: ata.mint,
            ata: ata.ata,
        };
        if ata.idempotent {
            cpi.idempotent()
                .rent_free(compressible_config, rent_sponsor, system_program)
                .invoke()?;
        } else {
            cpi.rent_free(compressible_config, rent_sponsor, system_program)
                .invoke()?;
        }
    }

    Ok(())
}
