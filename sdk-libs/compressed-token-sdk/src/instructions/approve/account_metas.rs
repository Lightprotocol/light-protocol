use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::instructions::CTokenDefaultAccounts;

/// Account metadata configuration for approve instruction
#[derive(Debug, Copy, Clone)]
pub struct ApproveMetaConfig {
    pub fee_payer: Option<Pubkey>,
    pub authority: Option<Pubkey>,
    pub delegated_compressed_account_merkle_tree: Pubkey,
    pub change_compressed_account_merkle_tree: Pubkey,
}

impl ApproveMetaConfig {
    /// Create a new ApproveMetaConfig for direct invocation
    pub fn new(
        fee_payer: Pubkey,
        authority: Pubkey,
        delegated_compressed_account_merkle_tree: Pubkey,
        change_compressed_account_merkle_tree: Pubkey,
    ) -> Self {
        Self {
            fee_payer: Some(fee_payer),
            authority: Some(authority),
            delegated_compressed_account_merkle_tree,
            change_compressed_account_merkle_tree,
        }
    }

    /// Create a new ApproveMetaConfig for client-side (CPI) usage
    pub fn new_client(
        delegated_compressed_account_merkle_tree: Pubkey,
        change_compressed_account_merkle_tree: Pubkey,
    ) -> Self {
        Self {
            fee_payer: None,
            authority: None,
            delegated_compressed_account_merkle_tree,
            change_compressed_account_merkle_tree,
        }
    }
}

/// Get the standard account metas for an approve instruction
/// Uses the GenericInstruction account structure for delegation operations
pub fn get_approve_instruction_account_metas(config: ApproveMetaConfig) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();

    // Calculate capacity based on whether fee_payer is provided
    // Base accounts: cpi_authority_pda + light_system_program + registered_program_pda +
    //                noop_program + account_compression_authority + account_compression_program +
    //                self_program + system_program + delegated_merkle_tree + change_merkle_tree
    let base_capacity = 10;

    // Direct invoke accounts: fee_payer + authority
    let fee_payer_capacity = if config.fee_payer.is_some() { 2 } else { 0 };

    let total_capacity = base_capacity + fee_payer_capacity;

    // Start building the account metas to match GenericInstruction structure
    let mut metas = Vec::with_capacity(total_capacity);

    // Add fee_payer and authority if provided (for direct invoke)
    if let Some(fee_payer) = config.fee_payer {
        let authority = config.authority.expect("Missing authority");
        metas.extend_from_slice(&[
            // fee_payer (mut, signer)
            AccountMeta::new(fee_payer, true),
            // authority (signer)
            AccountMeta::new_readonly(authority, true),
        ]);
    }

    // cpi_authority_pda
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.cpi_authority_pda,
        false,
    ));

    // light_system_program
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.light_system_program,
        false,
    ));

    // registered_program_pda
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.registered_program_pda,
        false,
    ));

    // noop_program
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.noop_program,
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

    // self_program (compressed token program)
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.self_program,
        false,
    ));

    // system_program
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.system_program,
        false,
    ));

    // delegated_compressed_account_merkle_tree (mut) - for the delegated output account
    metas.push(AccountMeta::new(
        config.delegated_compressed_account_merkle_tree,
        false,
    ));

    // change_compressed_account_merkle_tree (mut) - for the change output account
    metas.push(AccountMeta::new(
        config.change_compressed_account_merkle_tree,
        false,
    ));

    metas
}
