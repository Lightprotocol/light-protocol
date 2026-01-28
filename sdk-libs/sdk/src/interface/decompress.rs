//! SDK generic decompression functions.
//!
//! These functions are generic over account types and can be reused by the macro.
//! The decompress flow creates PDAs from compressed state (needs validity proof, packed data, seeds).

use anchor_lang::{
    prelude::*,
    solana_program::{clock::Clock, rent::Rent, sysvar::Sysvar},
};
use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext, with_account_info::CompressedAccountInfo,
};
use light_sdk_types::{
    cpi_context_write::CpiContextWriteAccounts,
    instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress, CpiSigner,
};
use light_token_interface::{
    instructions::{
        extensions::ExtensionInstructionData,
        transfer2::{CompressedTokenInstructionDataTransfer2, MultiInputTokenDataWithContext},
    },
    LIGHT_TOKEN_PROGRAM_ID, TRANSFER2,
};
use solana_instruction::Instruction;
use solana_program::program::invoke_signed;
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
        meta: &CompressedAccountMetaNoLamportsNoAddress, //TODO: pull into variant
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
    pub token_accounts_offset: u8,
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
    pub rent: &'a Rent,
    pub current_slot: u64,
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

    let cpi_accounts = CpiAccounts::new(
        fee_payer,
        &remaining_accounts[system_accounts_offset_usize..],
        cpi_signer,
    );

    // Build context struct with all needed data (includes internal vec)
    let mut decompress_ctx = DecompressCtx {
        program_id,
        cpi_accounts: &cpi_accounts,
        remaining_accounts,
        rent_sponsor,
        light_config: &light_config,
        rent: &rent,
        current_slot,
        compressed_account_infos: Vec::new(),
        in_token_data: Vec::new(),
        in_tlv: None,
        token_seeds: Vec::new(),
    };
    // TODO: check that lengths match
    let (pda_accounts, token_accounts) = params
        .accounts
        .split_at_checked(params.token_accounts_offset as usize)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let (pda_account_infos, token_account_infos) = remaining_accounts
        .split_at_checked(params.token_accounts_offset as usize)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // Process each account using trait dispatch on inner variant
    for (pda_account, pda_account_info) in pda_accounts.iter().zip(pda_account_infos) {
        // Dispatch via trait - implementation is in program's PackedProgramAccountVariant
        pda_account
            .data
            .decompress(&pda_account.meta, pda_account_info, &mut decompress_ctx)?;
    }
    // Process each account using trait dispatch on inner variant
    for (token_account, token_account_info) in token_accounts.iter().zip(token_account_infos) {
        // Dispatch via trait - implementation is in program's PackedProgramAccountVariant
        token_account.data.decompress(
            &token_account.meta,
            token_account_info,
            &mut decompress_ctx,
        )?;
    }

    let has_pda_accounts = !pda_accounts.is_empty();
    let has_token_accounts = !token_accounts.is_empty();

    if !has_pda_accounts {
        // CPI to Light System Program with proof
        if !has_token_accounts {
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
    }

    if has_token_accounts {
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
            compressions: None,
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

        let account_metas = remaining_accounts
            .iter()
            .map(|account| AccountMeta {
                pubkey: *account.key,
                is_signer: account.is_signer,
                is_writable: account.is_writable,
            })
            .collect::<Vec<_>>();
        let mut instruction_data = vec![TRANSFER2];
        cpi.serialize(&mut instruction_data).unwrap();
        let instruction = Instruction {
            program_id: LIGHT_TOKEN_PROGRAM_ID.into(),
            accounts: account_metas,
            data: instruction_data,
        };
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

    Ok(())
}
