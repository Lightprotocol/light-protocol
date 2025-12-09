//! Anchor integration helpers for Light Protocol compressed token operations.
//!
//! This module provides helper functions that are called by Anchor-generated code
//! to perform batched compressed token operations (CMint creation + mint_to actions).

use light_account_checks::AccountInfoTrait;
use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, traits::LightInstructionData,
};
use light_ctoken_interface::instructions::mint_action::{
    CompressedMintInstructionData, CpiContext as MintCpiContext,
    MintActionCompressedInstructionData, MintToCTokenAction,
};
use light_sdk::cpi::v2::CpiAccounts;
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::compressed_token::mint_action::MintActionMetaConfig;
use crate::ctoken::derive_cmint_from_spl_mint;

/// A mint action to be executed (recipient + amount).
#[derive(Clone, Debug)]
pub struct MintToAction<'info> {
    pub recipient: AccountInfo<'info>,
    pub amount: u64,
}

/// Input parameters for CMint finalization CPI.
pub struct CMintFinalizeParams<'c, 'info> {
    /// The compressed token program ID
    pub ctoken_program_id: Pubkey,
    /// CPI accounts for Light Protocol operations
    pub cpi_accounts: &'c CpiAccounts<'c, 'info>,
    /// SPL mint key (used to derive compressed mint address)
    pub spl_mint_key: Pubkey,
    /// Compressed mint instruction data (built from constraints with decimals, authorities, metadata)
    pub mint_data: CompressedMintInstructionData,
    /// Address tree pubkey index
    pub address_tree_idx: u8,
    /// Root index for address tree
    pub root_index: u16,
    /// Output state tree index
    pub output_state_tree_index: u8,
    /// Validity proof for the operation (already converted to CompressedProof)
    pub proof: Option<CompressedProof>,
    /// Number of CPDAs that were written to cpi_context before this mint
    pub cpda_count: u8,
    /// Queued mint_to actions
    pub mint_actions: Vec<MintToAction<'info>>,
    /// Mint signer key
    pub mint_signer_key: Pubkey,
    /// Fee payer key
    pub fee_payer_key: Pubkey,
    /// Authority key
    pub authority_key: Pubkey,
}

/// Build the CMint creation instruction with optional mint_to actions.
///
/// This function builds the instruction data and account metas for creating a
/// compressed mint and optionally minting tokens in a single CPI.
pub fn build_cmint_instruction<'info>(
    params: &CMintFinalizeParams<'_, 'info>,
) -> Result<(Instruction, Vec<AccountInfo<'info>>), ProgramError> {
    // Get tree accounts
    let tree_accounts = params
        .cpi_accounts
        .tree_accounts()
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let output_state_queue = *tree_accounts[params.output_state_tree_index as usize].key;
    let address_tree_pubkey = *tree_accounts[params.address_tree_idx as usize].key;

    // Derive compressed mint address
    let mint_compressed_address =
        derive_cmint_from_spl_mint(&params.spl_mint_key, &address_tree_pubkey);

    // Build instruction data
    let mut mint_instruction_data = MintActionCompressedInstructionData::new_mint(
        mint_compressed_address,
        params.root_index,
        params.proof.clone().unwrap_or_default(),
        params.mint_data.clone(),
    );

    // Add mint_to_ctoken actions
    for (i, action) in params.mint_actions.iter().enumerate() {
        mint_instruction_data = mint_instruction_data.with_mint_to_ctoken(MintToCTokenAction {
            account_index: i as u8,
            amount: action.amount,
        });
    }

    // Add CPI context to read CPDAs from cpi_context account
    let cpi_context = params
        .cpi_accounts
        .cpi_context()
        .map_err(|_| ProgramError::InvalidAccountData)?;

    mint_instruction_data = mint_instruction_data.with_cpi_context(MintCpiContext {
        address_tree_pubkey: address_tree_pubkey.to_bytes(),
        set_context: false,
        first_set_context: false,
        in_tree_index: params.address_tree_idx,
        in_queue_index: 0,
        out_queue_index: params.output_state_tree_index,
        token_out_queue_index: params.output_state_tree_index,
        assigned_account_index: params.cpda_count,
        read_only_address_trees: [0; 4],
    });

    // Build account metas
    let mut meta_config = MintActionMetaConfig::new_create_mint(
        params.fee_payer_key,
        params.authority_key,
        params.mint_signer_key,
        address_tree_pubkey,
        output_state_queue,
    );
    meta_config.cpi_context = Some(cpi_context.key().into());

    // Add ctoken accounts for recipients
    let ctoken_accounts: Vec<Pubkey> = params
        .mint_actions
        .iter()
        .map(|a| *a.recipient.key)
        .collect();
    meta_config = meta_config.with_ctoken_accounts(ctoken_accounts);

    let account_metas = meta_config.to_account_metas();

    // Serialize instruction data
    let data = mint_instruction_data
        .data()
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    let instruction = Instruction {
        program_id: params.ctoken_program_id,
        accounts: account_metas,
        data,
    };

    // Build account infos from cpi_accounts
    let mut account_infos = params.cpi_accounts.to_account_infos();

    // Add recipient accounts (avoiding duplicates)
    for action in &params.mint_actions {
        let recipient_key = action.recipient.key;
        let already_added = account_infos.iter().any(|ai| ai.key == recipient_key);
        if !already_added {
            account_infos.push(action.recipient.clone());
        }
    }

    Ok((instruction, account_infos))
}

/// Execute the CMint finalization CPI.
///
/// This is the main entry point called by Anchor-generated code. It:
/// 1. Builds the instruction and account infos
/// 2. Executes the CPI with the provided signer seeds
///
/// # Arguments
/// * `params` - CMint finalization parameters
/// * `additional_account_infos` - Additional accounts to include (cpi_authority, ctoken_program, etc.)
/// * `signer_seeds` - PDA signer seeds for invoke_signed
pub fn execute_cmint_finalize<'info>(
    params: CMintFinalizeParams<'_, 'info>,
    additional_account_infos: Vec<AccountInfo<'info>>,
    signer_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let (instruction, mut account_infos) = build_cmint_instruction(&params)?;

    // Add any additional account infos (cpi_authority, ctoken_program, etc.)
    account_infos.extend(additional_account_infos);

    // Execute CPI
    invoke_signed(&instruction, &account_infos, signer_seeds)?;

    Ok(())
}
