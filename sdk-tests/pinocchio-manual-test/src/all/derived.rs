//! Derived code for create_all instruction.
//!
//! This implements LightPreInit/LightFinalize for creating all account types:
//! - 2 PDAs (Borsh + ZeroCopy) via `invoke_write_to_cpi_context_first()`
//! - 1 Mint via `CreateMints` with cpi_context_offset
//! - 1 Token Vault via `CreateTokenAccountCpi`
//! - 1 ATA via `CreateTokenAtaCpi`

use light_account_pinocchio::{
    create_accounts, AtaInitParam, CreateMintsInput, LightAccount, LightFinalize, LightPreInit,
    LightSdkTypesError, PdaInitParam, SharedAccounts, SingleMintParams, TokenInitParam,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{
    CreateAllAccounts, CreateAllParams, ALL_MINT_SIGNER_SEED, ALL_TOKEN_VAULT_SEED,
};

// ============================================================================
// LightPreInit Implementation - Creates all accounts at START of instruction
// ============================================================================

impl LightPreInit<AccountInfo, CreateAllParams> for CreateAllAccounts<'_> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo],
        params: &CreateAllParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        const NUM_LIGHT_PDAS: usize = 2;
        const NUM_LIGHT_MINTS: usize = 1;
        const NUM_TOKENS: usize = 1;
        const NUM_ATAS: usize = 1;

        let authority_key = *self.authority.key();
        let mint_signer_key = *self.mint_signer.key();
        let mint_key = *self.mint.key();

        let mint_signer_seeds: &[&[u8]] = &[
            ALL_MINT_SIGNER_SEED,
            authority_key.as_ref(),
            &[params.mint_signer_bump],
        ];

        let vault_seeds: &[&[u8]] = &[
            ALL_TOKEN_VAULT_SEED,
            mint_key.as_ref(),
            &[params.token_vault_bump],
        ];

        // Capture references for the pda_setup closure
        let borsh_record = self.borsh_record;
        let zero_copy_record = self.zero_copy_record;

        create_accounts::<AccountInfo, NUM_LIGHT_PDAS, NUM_LIGHT_MINTS, NUM_TOKENS, NUM_ATAS, _>(
            [
                PdaInitParam {
                    account: self.borsh_record,
                },
                PdaInitParam {
                    account: self.zero_copy_record,
                },
            ],
            |light_config, current_slot| {
                // Set compression_info on the Borsh record
                {
                    let mut account_data = borsh_record
                        .try_borrow_mut_data()
                        .map_err(|_| LightSdkTypesError::Borsh)?;
                    let record =
                        crate::pda::MinimalRecord::mut_from_account_data(&mut account_data);
                    record.set_decompressed(light_config, current_slot);
                }
                // Set compression_info on the ZeroCopy record
                {
                    let mut account_data = zero_copy_record
                        .try_borrow_mut_data()
                        .map_err(|_| LightSdkTypesError::Borsh)?;
                    let record_bytes = &mut account_data
                        [8..8 + core::mem::size_of::<crate::account_loader::ZeroCopyRecord>()];
                    let record: &mut crate::account_loader::ZeroCopyRecord =
                        bytemuck::from_bytes_mut(record_bytes);
                    record.set_decompressed(light_config, current_slot);
                }
                Ok(())
            },
            Some(CreateMintsInput {
                params: [SingleMintParams {
                    decimals: 6,
                    mint_authority: authority_key,
                    mint_bump: None,
                    freeze_authority: None,
                    mint_seed_pubkey: mint_signer_key,
                    authority_seeds: None,
                    mint_signer_seeds: Some(mint_signer_seeds),
                    token_metadata: None,
                }],
                mint_seed_accounts: [self.mint_signers_slice[0]],
                mint_accounts: [self.mints_slice[0]],
            }),
            [TokenInitParam {
                account: self.token_vault,
                mint: self.mint,
                owner: *self.vault_owner.key(),
                seeds: vault_seeds,
            }],
            [AtaInitParam {
                ata: self.user_ata,
                owner: self.ata_owner,
                mint: self.mint,
                idempotent: false,
            }],
            &SharedAccounts {
                fee_payer: self.payer,
                cpi_signer: crate::LIGHT_CPI_SIGNER,
                proof: &params.create_accounts_proof,
                program_id: crate::ID,
                compression_config: Some(self.compression_config),
                compressible_config: Some(self.compressible_config),
                rent_sponsor: Some(self.rent_sponsor),
                cpi_authority: Some(self.cpi_authority),
                system_program: Some(self.system_program),
            },
            remaining_accounts,
        )
    }
}

// ============================================================================
// LightFinalize Implementation - No-op for this flow
// ============================================================================

impl LightFinalize<AccountInfo, CreateAllParams> for CreateAllAccounts<'_> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        _params: &CreateAllParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        // All accounts were created in light_pre_init
        Ok(())
    }
}
