use anchor_lang::prelude::*;

pub trait SystemProgramAccounts<'info> {
    fn get_registered_program_pda(&self) -> AccountInfo<'info>;
    fn get_noop_program(&self) -> AccountInfo<'info>;
    fn get_account_compression_authority(&self) -> AccountInfo<'info>;
    fn get_account_compression_program(&self) -> AccountInfo<'info>;
    fn get_system_program(&self) -> AccountInfo<'info>;
    fn get_sol_pool_pda(&self) -> Option<AccountInfo<'info>>;
    fn get_decompression_recipient(&self) -> Option<AccountInfo<'info>>;
    fn get_light_system_program(&self) -> AccountInfo<'info>;
    fn get_self_program(&self) -> AccountInfo<'info>;
}

pub trait CompressedCpiContextTrait<'info> {
    fn get_cpi_context(&self) -> Option<AccountInfo<'info>>;
}

pub trait CompressedTokenProgramAccounts<'info> {
    fn get_token_cpi_authority_pda(&self) -> AccountInfo<'info>;
    fn get_compressed_token_program(&self) -> AccountInfo<'info>;
    fn get_escrow_authority_pda(&self) -> AccountInfo<'info>;
    fn get_token_pool_pda(&self) -> AccountInfo<'info>;
    fn get_spl_token_program(&self) -> AccountInfo<'info>;
    fn get_compress_or_decompress_token_account(&self) -> Option<AccountInfo<'info>>;
}

pub trait SignerAccounts<'info> {
    fn get_fee_payer(&self) -> AccountInfo<'info>;
    fn get_authority(&self) -> AccountInfo<'info>;
    fn get_cpi_authority_pda(&self) -> AccountInfo<'info>;
}

// TODO: create macros and include all accounts which are required for mint to
pub trait MintToAccounts<'info> {
    fn get_mint(&self) -> AccountInfo<'info>;
}
