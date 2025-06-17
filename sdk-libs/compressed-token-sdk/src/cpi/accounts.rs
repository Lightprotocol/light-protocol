use solana_program::account_info::AccountInfo;

/// CPI Accounts for decompressing compressed token accounts.
pub struct CompressedTokenDecompressCpiAccounts<'a> {
    pub fee_payer: AccountInfo<'a>,
    pub authority: AccountInfo<'a>,
    pub cpi_authority_pda: AccountInfo<'a>,
    pub light_system_program: AccountInfo<'a>,
    pub registered_program_pda: AccountInfo<'a>,
    pub noop_program: AccountInfo<'a>,
    pub account_compression_authority: AccountInfo<'a>,
    pub account_compression_program: AccountInfo<'a>,
    pub self_program: AccountInfo<'a>,
    pub token_pool_pda: AccountInfo<'a>,
    pub decompress_destination: AccountInfo<'a>,
    pub token_program: AccountInfo<'a>,
    pub system_program: AccountInfo<'a>,
    pub state_merkle_tree: AccountInfo<'a>,
    pub queue: AccountInfo<'a>,
}
