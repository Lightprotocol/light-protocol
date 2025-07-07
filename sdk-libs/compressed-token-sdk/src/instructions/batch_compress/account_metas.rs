use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::instructions::CTokenDefaultAccounts;

/// Account metadata configuration for batch compress instruction
#[derive(Debug, Copy, Clone)]
pub struct BatchCompressMetaConfig {
    pub fee_payer: Option<Pubkey>,
    pub authority: Option<Pubkey>,
    pub token_pool_pda: Pubkey,
    pub sender_token_account: Pubkey,
    pub token_program: Pubkey,
    pub merkle_tree: Pubkey,
    pub sol_pool_pda: Option<Pubkey>,
}

impl BatchCompressMetaConfig {
    /// Create a new BatchCompressMetaConfig for direct invocation
    pub fn new(
        fee_payer: Pubkey,
        authority: Pubkey,
        token_pool_pda: Pubkey,
        sender_token_account: Pubkey,
        token_program: Pubkey,
        merkle_tree: Pubkey,
        with_lamports: bool,
    ) -> Self {
        let sol_pool_pda = if with_lamports {
            unimplemented!("TODO hardcode sol pool pda")
        } else {
            None
        };
        Self {
            fee_payer: Some(fee_payer),
            authority: Some(authority),
            token_pool_pda,
            sender_token_account,
            token_program,
            merkle_tree,
            sol_pool_pda,
        }
    }

    /// Create a new BatchCompressMetaConfig for client-side (CPI) usage
    pub fn new_client(
        token_pool_pda: Pubkey,
        sender_token_account: Pubkey,
        token_program: Pubkey,
        merkle_tree: Pubkey,
        with_lamports: bool,
    ) -> Self {
        let sol_pool_pda = if with_lamports {
            unimplemented!("TODO hardcode sol pool pda")
        } else {
            None
        };
        Self {
            fee_payer: None,
            authority: None,
            token_pool_pda,
            sender_token_account,
            token_program,
            merkle_tree,
            sol_pool_pda,
        }
    }
}

/// Get the standard account metas for a batch compress instruction
/// Matches the MintToInstruction account structure used by batch_compress
pub fn get_batch_compress_instruction_account_metas(
    config: BatchCompressMetaConfig,
) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();

    // Calculate capacity based on whether fee_payer is provided
    // Base accounts:   cpi_authority_pda + token_pool_pda + token_program + light_system_program +
    //                  registered_program_pda + noop_program + account_compression_authority +
    //                  account_compression_program + merkle_tree +
    //                  self_program + system_program + sender_token_account
    let base_capacity = 11;

    // Direct invoke accounts: fee_payer + authority + mint_placeholder + sol_pool_pda_or_placeholder
    let fee_payer_capacity = if config.fee_payer.is_some() { 4 } else { 0 };

    let total_capacity = base_capacity + fee_payer_capacity;

    // Start building the account metas to match MintToInstruction structure
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

    // mint: Option<UncheckedAccount> - Always None for batch_compress, so we add a placeholder
    if config.fee_payer.is_some() {
        metas.push(AccountMeta::new_readonly(
            default_pubkeys.compressed_token_program,
            false,
        ));
    }
    println!("config {:?}", config);
    println!("default_pubkeys {:?}", default_pubkeys);
    // token_pool_pda (mut)
    metas.push(AccountMeta::new(config.token_pool_pda, false));

    // token_program
    metas.push(AccountMeta::new_readonly(config.token_program, false));

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

    // merkle_tree (mut)
    metas.push(AccountMeta::new(config.merkle_tree, false));

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

    // sol_pool_pda (optional, mut) - add placeholder if None but fee_payer is present
    if let Some(sol_pool_pda) = config.sol_pool_pda {
        metas.push(AccountMeta::new(sol_pool_pda, false));
    } else if config.fee_payer.is_some() {
        metas.push(AccountMeta::new_readonly(
            default_pubkeys.compressed_token_program,
            false,
        ));
    }

    // sender_token_account (mut) - last account
    metas.push(AccountMeta::new(config.sender_token_account, false));

    metas
}
