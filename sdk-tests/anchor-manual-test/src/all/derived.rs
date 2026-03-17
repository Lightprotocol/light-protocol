//! Derived code for create_all instruction.
//!
//! This implements LightPreInit/LightFinalize for creating all account types:
//! - 2 PDAs (Borsh + ZeroCopy) via `invoke_write_to_cpi_context_first()`
//! - 1 Mint via `invoke_create_mints()` with cpi_context_offset
//! - 1 Token Vault via `CreateTokenAccountCpi`
//! - 1 ATA via `CreateTokenAtaCpi`

use anchor_lang::prelude::*;
use light_account::{
    create_accounts, AtaInitParam, CreateMintsInput, LightAccount, LightFinalize, LightPreInit,
    LightSdkTypesError, PdaInitParam, SharedAccounts, SingleMintParams, TokenInitParam,
};
use solana_account_info::AccountInfo;

use super::accounts::{
    CreateAllAccounts, CreateAllParams, ALL_MINT_SIGNER_SEED, ALL_TOKEN_VAULT_SEED,
};

// ============================================================================
// LightPreInit Implementation - Creates all accounts at START of instruction
// ============================================================================

impl<'info> LightPreInit<AccountInfo<'info>, CreateAllParams> for CreateAllAccounts<'info> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &CreateAllParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        const NUM_LIGHT_PDAS: usize = 2;
        const NUM_LIGHT_MINTS: usize = 1;
        const NUM_TOKENS: usize = 1;
        const NUM_ATAS: usize = 1;

        let authority_key = self.authority.key().to_bytes();
        let mint_signer_key = self.mint_signer.key().to_bytes();
        let mint_key = self.mint.key();

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

        let payer_info = self.payer.to_account_info();
        let borsh_record_info = self.borsh_record.to_account_info();
        let zero_copy_record_info = self.zero_copy_record.to_account_info();
        let mint_info = self.mint.to_account_info();
        let token_vault_info = self.token_vault.to_account_info();
        let user_ata_info = self.user_ata.to_account_info();
        let system_program_info = self.system_program.to_account_info();

        create_accounts::<
            AccountInfo<'info>,
            NUM_LIGHT_PDAS,
            NUM_LIGHT_MINTS,
            NUM_TOKENS,
            NUM_ATAS,
            _,
        >(
            [
                PdaInitParam {
                    account: &borsh_record_info,
                },
                PdaInitParam {
                    account: &zero_copy_record_info,
                },
            ],
            |light_config, current_slot| {
                // Set compression_info on the Borsh record
                self.borsh_record
                    .set_decompressed(light_config, current_slot);
                // Set compression_info on the ZeroCopy record
                {
                    let mut record = self
                        .zero_copy_record
                        .load_init()
                        .map_err(|_| LightSdkTypesError::Borsh)?;
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
                mint_seed_accounts: [self.mint_signer.to_account_info()],
                mint_accounts: [mint_info.clone()],
            }),
            [TokenInitParam {
                account: &token_vault_info,
                mint: &mint_info,
                owner: self.vault_owner.key.to_bytes(),
                seeds: vault_seeds,
            }],
            [AtaInitParam {
                ata: &user_ata_info,
                owner: &self.ata_owner,
                mint: &mint_info,
                idempotent: false,
            }],
            &SharedAccounts {
                fee_payer: &payer_info,
                cpi_signer: crate::LIGHT_CPI_SIGNER,
                proof: &params.create_accounts_proof,
                program_id: crate::LIGHT_CPI_SIGNER.program_id,
                compression_config: Some(&self.compression_config),
                compressible_config: Some(&self.compressible_config),
                rent_sponsor: Some(&self.rent_sponsor),
                cpi_authority: Some(&self.cpi_authority),
                system_program: Some(&system_program_info),
            },
            remaining_accounts,
        )
    }
}

// ============================================================================
// LightFinalize Implementation - No-op for this flow
// ============================================================================

impl<'info> LightFinalize<AccountInfo<'info>, CreateAllParams> for CreateAllAccounts<'info> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreateAllParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        // All accounts were created in light_pre_init
        Ok(())
    }
}
