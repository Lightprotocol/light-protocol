//! Runtime trait for compress_accounts_idempotent instruction.
use light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo;
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// Trait for compression context.
///
/// Programs implement this for their CompressAccountsIdempotent struct.
/// The macro generates this implementation automatically.
pub trait CompressContext<'info> {
    // Account accessors
    fn fee_payer(&self) -> &AccountInfo<'info>;
    fn config(&self) -> &AccountInfo<'info>;
    fn rent_sponsor(&self) -> &AccountInfo<'info>;
    fn ctoken_rent_sponsor(&self) -> &AccountInfo<'info>;
    fn compression_authority(&self) -> &AccountInfo<'info>;
    fn ctoken_compression_authority(&self) -> &AccountInfo<'info>;
    fn ctoken_program(&self) -> &AccountInfo<'info>;
    fn ctoken_cpi_authority(&self) -> &AccountInfo<'info>;

    /// Compress a single PDA account.
    ///
    /// Program-specific: handles discriminator matching and deserialization.
    fn compress_pda_account(
        &self,
        account_info: &AccountInfo<'info>,
        meta: &CompressedAccountMetaNoLamportsNoAddress,
        cpi_accounts: &crate::cpi::v2::CpiAccounts<'_, 'info>,
        compression_config: &crate::compressible::CompressibleConfig,
        program_id: &Pubkey,
    ) -> Result<Option<CompressedAccountInfo>, ProgramError>;
}
