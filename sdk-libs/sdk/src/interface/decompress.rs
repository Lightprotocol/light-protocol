//! SDK generic decompression functions.
//!
//! These functions are generic over account types and can be reused by the macro.
//! The decompress flow creates PDAs from compressed state (needs validity proof, packed data, seeds).

use anchor_lang::{
    prelude::*,
    solana_program::{clock::Clock, program::invoke_signed, rent::Rent, sysvar::Sysvar},
};
use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext, with_account_info::CompressedAccountInfo,
};
#[cfg(feature = "cpi-context")]
use light_sdk_types::cpi_context_write::CpiContextWriteAccounts;
use light_sdk_types::{
    cpi_accounts::CpiAccountsConfig, instruction::PackedStateTreeInfo, CpiSigner,
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, LIGHT_SYSTEM_PROGRAM_ID,
    REGISTERED_PROGRAM_PDA,
};
use light_token_interface::{
    instructions::{
        extensions::ExtensionInstructionData,
        transfer2::{
            CompressedTokenInstructionDataTransfer2, Compression, MultiInputTokenDataWithContext,
        },
    },
    CPI_AUTHORITY, LIGHT_TOKEN_PROGRAM_ID, TRANSFER2,
};
use solana_instruction::Instruction;
use solana_program_error::ProgramError;

use crate::{
    cpi::{v2::CpiAccounts, InvokeLightSystemProgram},
    instruction::ValidityProof,
    interface::{compression_info::CompressedAccountData, LightConfig},
    light_account_checks::account_iterator::AccountIterator,
};

// ============================================================================
// DecompressVariant Trait (implemented by program's PackedProgramAccountVariant)
// ============================================================================

/// Trait for packed program account variants that support decompression.
///
/// This trait is implemented by the program's `PackedProgramAccountVariant` enum
/// to handle type-specific dispatch during decompression.
///
/// MACRO-GENERATED: The implementation contains a match statement routing each
/// enum variant to the appropriate `prepare_account_for_decompression` call.
pub trait DecompressVariant<'info>: AnchorSerialize + AnchorDeserialize + Clone {
    /// Decompress this variant into a PDA account.
    ///
    /// The implementation should match on the enum variant and call
    /// `prepare_account_for_decompression::<SEED_COUNT, PackedVariantType>(packed, pda_account, ctx)`.
    fn decompress(
        &self,
        meta: &PackedStateTreeInfo,
        pda_account: &AccountInfo<'info>,
        ctx: &mut DecompressCtx<'_, 'info>,
    ) -> std::result::Result<(), ProgramError>;
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
    /// All account variants less than offset are pda acccounts.
    /// 255 if no token accounts
    pub token_accounts_offset: u8,
    /// Packed index of the output queue in remaining_accounts.
    pub output_queue_index: u8,
    /// Validity proof for compressed account verification
    pub proof: ValidityProof,
    /// Accounts to decompress - wrapped in CompressedAccountData for metadata
    pub accounts: Vec<CompressedAccountData<V>>,
}

/// Context struct holding all data needed for decompression.
/// Contains internal vec for collecting CompressedAccountInfo results.
pub struct DecompressCtx<'a, 'info> {
    pub program_id: &'a Pubkey,
    pub cpi_accounts: &'a CpiAccounts<'a, 'info>,
    pub remaining_accounts: &'a [AccountInfo<'info>],
    pub rent_sponsor: &'a AccountInfo<'info>,
    pub light_config: &'a LightConfig,
    /// Token (ctoken) rent sponsor for creating token accounts
    pub ctoken_rent_sponsor: &'a AccountInfo<'info>,
    /// Token (ctoken) compressible config for creating token accounts
    pub ctoken_compressible_config: &'a AccountInfo<'info>,
    pub rent: &'a Rent,
    pub current_slot: u64,
    /// Packed index of the output queue in remaining_accounts.
    pub output_queue_index: u8,
    /// Internal vec - dispatch functions push results here
    pub compressed_account_infos: Vec<CompressedAccountInfo>,
    pub in_token_data: Vec<MultiInputTokenDataWithContext>,
    pub in_tlv: Option<Vec<Vec<ExtensionInstructionData>>>,
    pub token_seeds: Vec<Vec<u8>>,
}

// ============================================================================
// Processor Function
// ============================================================================

/// Remaining accounts layout:
/// [0]: fee_payer (Signer, mut)
/// [1]: config (LightConfig PDA)
/// [2]: rent_sponsor (mut)
/// [system_accounts_offset..]: Light system accounts for CPI
/// [remaining_accounts.len() - num_pda_accounts..]: PDA accounts to decompress
///
/// Runtime processor - handles all the plumbing, dispatches via DecompressVariant trait.
///
/// **Takes raw instruction data** and deserializes internally - minimizes macro code.
/// **Uses only remaining_accounts** - no Context struct needed.
/// **Generic over V** - the program's `PackedProgramAccountVariant` enum.
pub fn process_decompress_pda_accounts_idempotent<'info, V>(
    remaining_accounts: &[AccountInfo<'info>],
    instruction_data: &[u8],
    cpi_signer: CpiSigner,
    program_id: &Pubkey,
) -> std::result::Result<(), ProgramError>
where
    V: DecompressVariant<'info>,
{
    // Deserialize params internally
    let params = DecompressIdempotentParams::<V>::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Extract and validate accounts using AccountIterator
    let mut account_iter = AccountIterator::new(remaining_accounts);
    let fee_payer = account_iter
        .next_signer_mut("fee_payer")
        .map_err(ProgramError::from)?;
    let config = account_iter
        .next_non_mut("config")
        .map_err(ProgramError::from)?;
    let rent_sponsor = account_iter
        .next_mut("rent_sponsor")
        .map_err(ProgramError::from)?;

    // Load and validate config
    let light_config = LightConfig::load_checked(config, program_id)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let rent = Rent::get()?;
    let current_slot = Clock::get()?.slot;

    let system_accounts_offset_usize = params.system_accounts_offset as usize;
    if system_accounts_offset_usize > remaining_accounts.len() {
        return Err(ProgramError::InvalidInstructionData);
    }
    let (pda_accounts, token_accounts) = params
        .accounts
        .split_at_checked(params.token_accounts_offset as usize)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // PDA and token account infos are at the tail of remaining_accounts.
    let num_hot_accounts = params.accounts.len();
    let hot_accounts_start = remaining_accounts
        .len()
        .checked_sub(num_hot_accounts)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let hot_account_infos = &remaining_accounts[hot_accounts_start..];
    let (pda_account_infos, token_account_infos) = hot_account_infos
        .split_at_checked(params.token_accounts_offset as usize)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let has_pda_accounts = !pda_accounts.is_empty();
    let has_token_accounts = !token_accounts.is_empty();
    let cpi_context = has_pda_accounts && has_token_accounts;
    let config = CpiAccountsConfig {
        sol_compression_recipient: false,
        sol_pool_pda: false,
        cpi_context,
        cpi_signer,
    };
    let cpi_accounts = CpiAccounts::new_with_config(
        fee_payer,
        &remaining_accounts[system_accounts_offset_usize..],
        config,
    );

    // Token (ctoken) accounts layout in remaining_accounts:
    // [0]fee_payer, [1]pda_config, [2]pda_rent_sponsor, [3]ctoken_rent_sponsor,
    // [4]light_token_program, [5]cpi_authority, [6]ctoken_compressible_config
    let ctoken_rent_sponsor = remaining_accounts
        .get(3)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let ctoken_compressible_config = remaining_accounts
        .get(6)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // Build context struct with all needed data (includes internal vec)
    let mut decompress_ctx = DecompressCtx {
        program_id,
        cpi_accounts: &cpi_accounts,
        remaining_accounts,
        rent_sponsor,
        light_config: &light_config,
        ctoken_rent_sponsor,
        ctoken_compressible_config,
        rent: &rent,
        current_slot,
        output_queue_index: params.output_queue_index,
        compressed_account_infos: Vec::new(),
        in_token_data: Vec::new(),
        in_tlv: None,
        token_seeds: Vec::new(),
    };

    // Process each account using trait dispatch on inner variant
    for (pda_account, pda_account_info) in pda_accounts.iter().zip(pda_account_infos) {
        pda_account.data.decompress(
            &pda_account.tree_info,
            pda_account_info,
            &mut decompress_ctx,
        )?;
    }
    // Process token accounts
    for (token_account, token_account_info) in token_accounts.iter().zip(token_account_infos) {
        token_account.data.decompress(
            &token_account.tree_info,
            token_account_info,
            &mut decompress_ctx,
        )?;
    }

    if has_pda_accounts {
        // CPI to Light System Program with proof
        #[cfg(feature = "cpi-context")]
        let pda_only = !cpi_context;
        #[cfg(not(feature = "cpi-context"))]
        let pda_only = true;

        if pda_only {
            // Manual construction to avoid extra allocations
            let instruction_data = light_compressed_account::instruction_data::with_account_info::InstructionDataInvokeCpiWithAccountInfo {
                mode: 1,
                bump: cpi_signer.bump,
                invoking_program_id: cpi_signer.program_id.into(),
                compress_or_decompress_lamports: 0,
                is_compress: false,
                with_cpi_context: false,
                with_transaction_hash: false,
                cpi_context: CompressedCpiContext::default(),
                proof: params.proof.0,
                new_address_params: Vec::new(),
                account_infos: decompress_ctx.compressed_account_infos,
                read_only_addresses: Vec::new(),
                read_only_accounts: Vec::new(),
            };
            instruction_data.invoke(cpi_accounts.clone())?;
        } else {
            #[cfg(feature = "cpi-context")]
            {
                // PDAs + tokens - write to CPI context first, tokens will execute
                let authority = cpi_accounts
                    .authority()
                    .map_err(|_| ProgramError::MissingRequiredSignature)?;
                let cpi_context_account = cpi_accounts
                    .cpi_context()
                    .map_err(|_| ProgramError::MissingRequiredSignature)?;
                let system_cpi_accounts = CpiContextWriteAccounts {
                    fee_payer,
                    authority,
                    cpi_context: cpi_context_account,
                    cpi_signer,
                };

                // Manual construction to avoid extra allocations
                let instruction_data = light_compressed_account::instruction_data::with_account_info::InstructionDataInvokeCpiWithAccountInfo {
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
                    account_infos: decompress_ctx.compressed_account_infos,
                    read_only_addresses: Vec::new(),
                    read_only_accounts: Vec::new(),
                };
                instruction_data.invoke_write_to_cpi_context_first(system_cpi_accounts)?;
            }
            #[cfg(not(feature = "cpi-context"))]
            {
                return Err(ProgramError::InvalidInstructionData);
            }
        }
    }

    if has_token_accounts {
        let mut compressions = Vec::new();
        // Assumes is compressed to pubkey.
        decompress_ctx
            .in_token_data
            .iter()
            .for_each(|a| compressions.push(Compression::decompress(a.amount, a.mint, a.owner)));
        let mut cpi = CompressedTokenInstructionDataTransfer2 {
            with_transaction_hash: false,
            in_token_data: decompress_ctx.in_token_data.clone(),
            in_tlv: decompress_ctx.in_tlv.clone(),
            with_lamports_change_account_merkle_tree_index: false,
            lamports_change_account_merkle_tree_index: 0,
            lamports_change_account_owner_index: 0,
            output_queue: 0,
            max_top_up: 0,
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
            )
        }

        // Build Transfer2 account_metas in the order the handler expects:
        // [0] light_system_program (readonly)
        // [1] fee_payer (signer, writable)
        // [2] cpi_authority_pda (readonly)
        // [3] registered_program_pda (readonly)
        // [4] account_compression_authority (readonly)
        // [5] account_compression_program (readonly)
        // [6] system_program (readonly)
        // [7] cpi_context (optional, writable)
        // [N+] packed_accounts
        let mut account_metas = vec![
            AccountMeta::new_readonly(Pubkey::new_from_array(LIGHT_SYSTEM_PROGRAM_ID), false),
            AccountMeta::new(*fee_payer.key, true),
            AccountMeta::new_readonly(Pubkey::new_from_array(CPI_AUTHORITY), false),
            AccountMeta::new_readonly(Pubkey::new_from_array(REGISTERED_PROGRAM_PDA), false),
            AccountMeta::new_readonly(
                Pubkey::new_from_array(ACCOUNT_COMPRESSION_AUTHORITY_PDA),
                false,
            ),
            AccountMeta::new_readonly(
                Pubkey::new_from_array(ACCOUNT_COMPRESSION_PROGRAM_ID),
                false,
            ),
            AccountMeta::new_readonly(Pubkey::default(), false),
        ];
        if cpi_context {
            let cpi_ctx = cpi_accounts
                .cpi_context()
                .map_err(|_| ProgramError::NotEnoughAccountKeys)?;
            account_metas.push(AccountMeta::new(*cpi_ctx.key, false));
        }
        let transfer2_packed_start = account_metas.len();
        let packed_accounts_offset =
            system_accounts_offset_usize + cpi_accounts.system_accounts_end_offset();
        for account in &remaining_accounts[packed_accounts_offset..] {
            account_metas.push(AccountMeta {
                pubkey: *account.key,
                is_signer: account.is_signer,
                is_writable: account.is_writable,
            });
        }
        cpi.in_token_data.iter().for_each(|data| {
            account_metas[data.owner as usize + transfer2_packed_start].is_signer = true;
        });
        let mut instruction_data = vec![TRANSFER2];
        cpi.serialize(&mut instruction_data).unwrap();
        let instruction = Instruction {
            program_id: LIGHT_TOKEN_PROGRAM_ID.into(),
            accounts: account_metas,
            data: instruction_data,
        };
        // For ATAs, no PDA signing is needed (wallet owner signed at transaction level).
        // For regular token accounts, use invoke_signed with PDA seeds.
        if decompress_ctx.token_seeds.is_empty() {
            // All tokens are ATAs - use regular invoke (no PDA signing needed)
            anchor_lang::solana_program::program::invoke(&instruction, remaining_accounts)?;
        } else {
            // At least one regular token account - use invoke_signed with PDA seeds
            let signer_seed_refs: Vec<&[u8]> = decompress_ctx
                .token_seeds
                .iter()
                .map(|s| s.as_slice())
                .collect();

            invoke_signed(
                &instruction,
                remaining_accounts,
                &[signer_seed_refs.as_slice()],
            )?;
        }
    }

    Ok(())
}
