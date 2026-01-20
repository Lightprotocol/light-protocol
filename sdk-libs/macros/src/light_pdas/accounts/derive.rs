//! Orchestration layer for LightAccounts derive macro.
//!
//! This module coordinates code generation by combining:
//! - PDA block generation from `pda.rs`
//! - Mint action invocation from `mint.rs`
//! - Parsing results from `parse.rs`
//!
//! Design for mints:
//! - At mint init, we CREATE + DECOMPRESS atomically
//! - After init, the Mint should always be in decompressed/"hot" state
//!
//! Flow for PDAs + mints:
//! 1. Pre-init: ALL compression logic executes here
//!    a. Write PDAs to CPI context
//!    b. Invoke mint_action with decompress + CPI context
//!    c. Mint is now "hot" and usable
//! 2. Instruction body: Can use hot Mint (mintTo, transfers, etc.)
//! 3. Finalize: No-op (all work done in pre_init)

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use super::builder::LightAccountsBuilder;

/// Main orchestration - shows the high-level flow clearly.
pub(super) fn derive_light_accounts(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let builder = LightAccountsBuilder::parse(input)?;
    builder.validate()?;

    // No instruction args = no-op impls (backwards compatibility)
    if !builder.has_instruction_args() {
        return builder.generate_noop_impls();
    }

    // Generate pre_init body based on what fields we have
    let pre_init = if builder.has_pdas() && builder.has_mints() {
        builder.generate_pre_init_pdas_and_mints()?
    } else if builder.has_mints() {
        builder.generate_pre_init_mints_only()?
    } else if builder.has_pdas() {
        builder.generate_pre_init_pdas_only()?
    } else {
        quote! { Ok(false) }
    };

    // Generate trait implementations
    let pre_init_impl = builder.generate_pre_init_impl(pre_init)?;

    // Generate finalize body - token accounts and ATAs are created here
    let finalize_body = if builder.needs_token_finalize() {
        builder.generate_token_finalize_body()
    } else {
        quote! { Ok(()) }
    };
    let finalize_impl = builder.generate_finalize_impl(finalize_body)?;

    Ok(quote! {
        #pre_init_impl
        #finalize_impl
    })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_token_account_with_init_generates_create_cpi() {
        // Token account with init should generate CreateTokenAccountCpi in finalize
        let input: DeriveInput = parse_quote! {
            #[instruction(params: CreateVaultParams)]
            pub struct CreateVault<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,

                #[light_account(init, token, authority = [b"authority"], mint = my_mint, owner = fee_payer)]
                pub vault: Account<'info, CToken>,

                pub light_token_compressible_config: Account<'info, CompressibleConfig>,
                pub light_token_rent_sponsor: Account<'info, RentSponsor>,
                pub light_token_cpi_authority: AccountInfo<'info>,
            }
        };

        let result = derive_light_accounts(&input);
        assert!(result.is_ok(), "Token account derive should succeed");

        let output = result.unwrap().to_string();

        // Verify finalize generates token account creation
        assert!(
            output.contains("LightFinalize"),
            "Should generate LightFinalize impl"
        );
        assert!(
            output.contains("CreateTokenAccountCpi"),
            "Should generate CreateTokenAccountCpi call"
        );
        assert!(
            output.contains("rent_free"),
            "Should call rent_free on CreateTokenAccountCpi"
        );
        assert!(
            output.contains("invoke_signed"),
            "Should call invoke_signed with seeds"
        );
    }

    #[test]
    fn test_ata_with_init_generates_create_cpi() {
        // ATA with init should generate create_associated_token_account_idempotent in finalize
        let input: DeriveInput = parse_quote! {
            #[instruction(params: CreateAtaParams)]
            pub struct CreateAta<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,

                #[light_account(init, associated_token, owner = wallet, mint = my_mint)]
                pub user_ata: Account<'info, CToken>,

                pub wallet: AccountInfo<'info>,
                pub my_mint: AccountInfo<'info>,
                pub light_token_compressible_config: Account<'info, CompressibleConfig>,
                pub light_token_rent_sponsor: Account<'info, RentSponsor>,
            }
        };

        let result = derive_light_accounts(&input);
        assert!(result.is_ok(), "ATA derive should succeed");

        let output = result.unwrap().to_string();

        // Verify finalize generates ATA creation
        assert!(
            output.contains("LightFinalize"),
            "Should generate LightFinalize impl"
        );
        assert!(
            output.contains("CreateTokenAtaCpi"),
            "Should generate CreateTokenAtaCpi call"
        );
    }

    #[test]
    fn test_token_mark_only_generates_noop_finalize() {
        // Token without init should NOT generate creation code (mark-only mode)
        // Mark-only returns None from parsing, so token_account_fields is empty
        let input: DeriveInput = parse_quote! {
            #[instruction(params: UseVaultParams)]
            pub struct UseVault<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,

                // Mark-only: no init keyword
                #[light_account(token, authority = [b"authority"])]
                pub vault: Account<'info, CToken>,
            }
        };

        let result = derive_light_accounts(&input);
        assert!(result.is_ok(), "Mark-only token derive should succeed");

        let output = result.unwrap().to_string();

        // Mark-only should NOT have token account creation
        assert!(
            !output.contains("CreateTokenAccountCpi"),
            "Mark-only should NOT generate CreateTokenAccountCpi"
        );

        // Should still have LightFinalize but with Ok(())
        assert!(
            output.contains("LightFinalize"),
            "Should still generate LightFinalize impl"
        );
    }

    #[test]
    fn test_mixed_token_and_ata_generates_both() {
        // Mixed token account + ATA should generate both creation codes
        let input: DeriveInput = parse_quote! {
            #[instruction(params: CreateBothParams)]
            pub struct CreateBoth<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,

                #[light_account(init, token, authority = [b"authority"], mint = my_mint, owner = fee_payer)]
                pub vault: Account<'info, CToken>,

                #[light_account(init, associated_token, owner = wallet, mint = my_mint)]
                pub user_ata: Account<'info, CToken>,

                pub wallet: AccountInfo<'info>,
                pub my_mint: AccountInfo<'info>,
                pub light_token_compressible_config: Account<'info, CompressibleConfig>,
                pub light_token_rent_sponsor: Account<'info, RentSponsor>,
                pub light_token_cpi_authority: AccountInfo<'info>,
            }
        };

        let result = derive_light_accounts(&input);
        assert!(result.is_ok(), "Mixed token+ATA derive should succeed");

        let output = result.unwrap().to_string();

        // Should have both creation types
        assert!(
            output.contains("CreateTokenAccountCpi"),
            "Should generate CreateTokenAccountCpi for vault"
        );
        assert!(
            output.contains("CreateTokenAtaCpi"),
            "Should generate CreateTokenAtaCpi for ATA"
        );
    }

    #[test]
    fn test_no_instruction_args_generates_noop() {
        // No #[instruction] attribute should generate no-op impls
        let input: DeriveInput = parse_quote! {
            pub struct NoInstruction<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,
            }
        };

        let result = derive_light_accounts(&input);
        assert!(result.is_ok(), "No instruction args should succeed");

        let output = result.unwrap().to_string();

        // Should generate no-op impls with () param type
        assert!(
            output.contains("LightPreInit"),
            "Should generate LightPreInit impl"
        );
        assert!(
            output.contains("LightFinalize"),
            "Should generate LightFinalize impl"
        );
        // No-op returns Ok(false) in pre_init and Ok(()) in finalize
        assert!(
            output.contains("Ok (false)") || output.contains("Ok(false)"),
            "Should return Ok(false) in pre_init"
        );
    }
}
