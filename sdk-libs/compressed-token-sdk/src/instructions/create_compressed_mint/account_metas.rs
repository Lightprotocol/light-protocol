use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::instructions::CTokenDefaultAccounts;

/// Account metadata configuration for create compressed mint instruction
#[derive(Debug, Copy, Clone)]
pub struct CreateCompressedMintMetaConfig {
    pub fee_payer: Option<Pubkey>,
    pub mint_signer: Option<Pubkey>,
    pub address_tree_pubkey: Pubkey,
    pub output_queue: Pubkey,
}

impl CreateCompressedMintMetaConfig {
    /// Create a new CreateCompressedMintMetaConfig for direct invocation
    pub fn new(
        fee_payer: Pubkey,
        mint_signer: Pubkey,
        address_tree_pubkey: Pubkey,
        output_queue: Pubkey,
    ) -> Self {
        Self {
            fee_payer: Some(fee_payer),
            mint_signer: Some(mint_signer),
            address_tree_pubkey,
            output_queue,
        }
    }

    /// Create a new CreateCompressedMintMetaConfig for client-side (CPI) usage
    pub fn new_client(
        mint_seed: Pubkey,
        address_tree_pubkey: Pubkey,
        output_queue: Pubkey,
    ) -> Self {
        Self {
            fee_payer: None,
            mint_signer: Some(mint_seed),
            address_tree_pubkey,
            output_queue,
        }
    }
}

/// Get the standard account metas for a create compressed mint instruction
pub fn get_create_compressed_mint_instruction_account_metas(
    config: CreateCompressedMintMetaConfig,
) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();

    // Calculate capacity based on whether fee_payer is provided
    // Base accounts: light_system_program + cpi_authority_pda + registered_program_pda +
    //                noop_program + account_compression_authority + account_compression_program +
    //                self_program + system_program + address_merkle_tree + output_queue
    let base_capacity = 10;

    // Direct invoke accounts: mint_signer + fee_payer
    let direct_invoke_capacity = if config.fee_payer.is_some() { 2 } else { 0 };

    let total_capacity = base_capacity + direct_invoke_capacity;

    let mut metas = Vec::with_capacity(total_capacity);

    // Add mint_signer and fee_payer if provided (for direct invoke)
    if let Some(mint_signer) = config.mint_signer {
        metas.push(AccountMeta::new_readonly(mint_signer, true));
    }

    // light_system_program
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.light_system_program,
        false,
    ));

    // Add fee_payer if provided (for direct invoke)
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

    // address_merkle_tree (mutable)
    metas.push(AccountMeta::new(config.address_tree_pubkey, false));

    // output_queue (mutable)
    metas.push(AccountMeta::new(config.output_queue, false));

    metas
}
