//! Accounts module for single-pda-test.

use crate::{init::prepare_compressed_account_on_init, state::MinimalRecord};
use anchor_lang::prelude::*;
use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext, with_account_info::InstructionDataInvokeCpiWithAccountInfo,
};
use light_compressible::CreateAccountsProof;
use light_sdk::{
    cpi::{v2::CpiAccounts, CpiAccountsConfig, InvokeLightSystemProgram},
    error::LightSdkError,
    instruction::PackedAddressTreeInfoExt,
    interface::{LightFinalize, LightPreInit},
    sdk_types::CpiContextWriteAccounts,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreatePdaParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Minimal accounts struct for testing single PDA creation.
#[derive(Accounts)]
#[instruction(params: CreatePdaParams)]
pub struct CreatePda<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + MinimalRecord::INIT_SPACE,
        seeds = [b"minimal_record", params.owner.as_ref()],
        bump,
    )]
    // #[light_account(init)]
    pub record: Account<'info, MinimalRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Manual LightPreInit Implementation
// ============================================================================

impl<'info> LightPreInit<'info, CreatePdaParams> for CreatePda<'info> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &CreatePdaParams,
    ) -> std::result::Result<bool, LightSdkError> {
        use solana_program_error::ProgramError;

        // 1. Build CPI accounts (slice remaining_accounts at system_accounts_offset)
        let system_accounts_offset = params.create_accounts_proof.system_accounts_offset as usize;
        if remaining_accounts.len() < system_accounts_offset {
            return Err(LightSdkError::FewerAccountsThanSystemAccounts);
        }
        let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
        let cpi_accounts = CpiAccounts::new_with_config(
            &self.fee_payer,
            &remaining_accounts[system_accounts_offset..],
            config,
        );

        // 2. Get address tree pubkey from packed tree info
        let address_tree_info = &params.create_accounts_proof.address_tree_info;
        let address_tree_pubkey = address_tree_info
            .get_tree_pubkey(&cpi_accounts)
            .map_err(|_| LightSdkError::from(ProgramError::InvalidAccountData))?;
        let output_tree_index = params.create_accounts_proof.output_state_tree_index;
        let current_account_index: u8 = 0;
        // Is true if the instruction creates 1 or more light mints in addition to 1 or more light pda accounts.
        const WITH_CPI_CONTEXT: bool = false;
        // Is first if the instruction creates 1 or more light mints in addition to 1 or more light pda accounts.
        let cpi_context = if WITH_CPI_CONTEXT {
            CompressedCpiContext::first()
        } else {
            CompressedCpiContext::default()
        };
        const NUM_LIGHT_PDAS: usize = 1;
        let mut new_address_params = Vec::with_capacity(NUM_LIGHT_PDAS);
        let mut account_infos = Vec::with_capacity(NUM_LIGHT_PDAS);

        // 3. Prepare compressed account using helper function
        let pda_pubkey = self.record.key();
        prepare_compressed_account_on_init(
            &pda_pubkey,
            &address_tree_pubkey,
            address_tree_info,
            output_tree_index,
            current_account_index,
            &crate::ID,
            &mut new_address_params,
            &mut account_infos,
        )?;
        // current_account_index += 1;
        // For multiple accounts, repeat the pattern:
        // let prepared2 = prepare_compressed_account_on_init(..., current_account_index, ...)?;
        // current_account_index += 1;

        // 4. Build instruction data manually (no builder pattern)
        let instruction_data = InstructionDataInvokeCpiWithAccountInfo {
            mode: 1, // V2 mode
            bump: crate::LIGHT_CPI_SIGNER.bump,
            invoking_program_id: crate::LIGHT_CPI_SIGNER.program_id.into(),
            compress_or_decompress_lamports: 0,
            is_compress: false,
            with_cpi_context: WITH_CPI_CONTEXT,
            with_transaction_hash: false,
            cpi_context,
            proof: params.create_accounts_proof.proof.0,
            new_address_params,
            account_infos,
            read_only_addresses: vec![],
            read_only_accounts: vec![],
        };
        if !WITH_CPI_CONTEXT {
            // 5. Invoke Light System Program CPI
            instruction_data
                .invoke(cpi_accounts)
                .map_err(LightSdkError::from)?;
        } else {
            // For flows that combine light mints with light PDAs, write to CPI context first.
            // The authority and cpi_context accounts must be provided in remaining_accounts.
            let cpi_context_accounts = CpiContextWriteAccounts {
                fee_payer: cpi_accounts.fee_payer(),
                authority: cpi_accounts.authority().map_err(LightSdkError::from)?,
                cpi_context: cpi_accounts.cpi_context().map_err(LightSdkError::from)?,
                cpi_signer: crate::LIGHT_CPI_SIGNER,
            };
            instruction_data
                .invoke_write_to_cpi_context_first(cpi_context_accounts)
                .map_err(LightSdkError::from)?;
        }

        Ok(false) // No mints, so no CPI context write
    }
}

// ============================================================================
// Manual LightFinalize Implementation (no-op for PDA-only flow)
// ============================================================================

impl<'info> LightFinalize<'info, CreatePdaParams> for CreatePda<'info> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreatePdaParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkError> {
        // No-op for PDA-only flow - compression CPI already executed in light_pre_init
        Ok(())
    }
}
