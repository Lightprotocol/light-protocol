use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::instructions::CTokenDefaultAccounts;

/// Configuration for generating account metas for update compressed mint instruction
#[derive(Debug, Clone)]
pub struct UpdateCompressedMintMetaConfig {
    pub fee_payer: Option<Pubkey>,
    pub authority: Option<Pubkey>,
    pub in_merkle_tree: Pubkey,
    pub in_output_queue: Pubkey,
    pub out_output_queue: Pubkey,
    pub with_cpi_context: bool,
}

/// Generates account metas for the update compressed mint instruction
/// Following the same pattern as other compressed token instructions
pub fn get_update_compressed_mint_instruction_account_metas(
    config: UpdateCompressedMintMetaConfig,
) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();

    let mut metas = Vec::new();

    // First two accounts are static non-CPI accounts as expected by CPI_ACCOUNTS_OFFSET = 2
    // light_system_program (always required)
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.light_system_program,
        false,
    ));

    // authority (signer, always required)
    if let Some(authority) = config.authority {
        metas.push(AccountMeta::new_readonly(authority, true));
    }

    if config.with_cpi_context {
        // CPI context accounts - similar to other CPI instructions
        // TODO: Add CPI context specific accounts when needed
    } else {
        // LightSystemAccounts (6 accounts)
        // fee_payer (signer, mutable)
        if let Some(fee_payer) = config.fee_payer {
            metas.push(AccountMeta::new(fee_payer, true));
        }

        // cpi_authority_pda
        metas.push(AccountMeta::new_readonly(
            default_pubkeys.cpi_authority_pda,
            false,
        ));

        // registered_program_pda
        metas.push(AccountMeta::new_readonly(
            default_pubkeys.registered_program_pda,
            false,
        ));

        // account_compression_authority
        metas.push(AccountMeta::new_readonly(
            default_pubkeys.account_compression_authority,
            false,
        ));

        // account_compression_program
        metas.push(AccountMeta::new_readonly(
            default_pubkeys.account_compression_program,
            false,
        ));

        // system_program
        metas.push(AccountMeta::new_readonly(
            default_pubkeys.system_program,
            false,
        ));

        // UpdateOneCompressedAccountTreeAccounts (3 accounts)
        // in_merkle_tree (mutable)
        metas.push(AccountMeta::new(config.in_merkle_tree, false));

        // in_output_queue (mutable)
        metas.push(AccountMeta::new(config.in_output_queue, false));

        // out_output_queue (mutable)
        metas.push(AccountMeta::new(config.out_output_queue, false));
    }

    metas
}
