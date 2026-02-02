//! Builder for RentFree derive macro code generation.
//!
//! Encapsulates parsed struct data and resolved infrastructure fields,
//! providing methods for validation, querying, and code generation.

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use super::{
    mint::{InfraRefs, LightMintsBuilder},
    parse::ParsedLightAccountsStruct,
    pda::{generate_pda_compress_blocks, generate_rent_reimbursement_block},
    token::TokenAccountsBuilder,
    validation::{validate_struct, ValidationContext},
};

/// Builder for RentFree derive macro code generation.
///
/// Encapsulates parsed struct data and resolved infrastructure fields,
/// providing methods for validation, querying, and code generation.
pub(super) struct LightAccountsBuilder {
    parsed: ParsedLightAccountsStruct,
    infra: InfraRefs,
}

impl LightAccountsBuilder {
    /// Parse a DeriveInput and construct the builder.
    pub fn parse(input: &DeriveInput) -> Result<Self, syn::Error> {
        let parsed = super::parse::parse_light_accounts_struct(input)?;
        let infra = InfraRefs::from_parsed(&parsed.infra_fields);
        Ok(Self { parsed, infra })
    }

    /// Get the first instruction argument, returning an error if missing.
    fn get_first_instruction_arg(&self) -> Result<&super::parse::InstructionArg, syn::Error> {
        self.parsed
            .instruction_args
            .as_ref()
            .and_then(|args| args.first())
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    &self.parsed.struct_name,
                    "Missing #[instruction(...)] attribute with at least one parameter",
                )
            })
    }

    /// Get the expression to access CreateAccountsProof.
    ///
    /// Returns either:
    /// - `proof_ident` (direct) if CreateAccountsProof is passed as a direct argument
    /// - `params.create_accounts_proof` (nested) if nested inside a params struct
    fn get_proof_access(&self) -> Result<TokenStream, syn::Error> {
        if let Some(ref proof_ident) = self.parsed.direct_proof_arg {
            Ok(quote! { #proof_ident })
        } else {
            let first_arg = self.get_first_instruction_arg()?;
            let params_ident = &first_arg.name;
            Ok(quote! { #params_ident.create_accounts_proof })
        }
    }

    /// Validate constraints using the struct-level validation module.
    pub fn validate(&self) -> Result<(), syn::Error> {
        let ctx = ValidationContext {
            struct_name: &self.parsed.struct_name,
            has_pdas: self.has_pdas(),
            has_mints: self.has_mints(),
            has_tokens: self.has_token_accounts(),
            has_tokens_with_init: self.has_token_accounts_with_init(),
            has_atas: self.has_atas(),
            has_atas_with_init: self.has_atas_with_init(),
            has_fee_payer: self.parsed.infra_fields.fee_payer.is_some(),
            has_compression_config: self.parsed.infra_fields.compression_config.is_some(),
            has_pda_rent_sponsor: self.parsed.infra_fields.pda_rent_sponsor.is_some(),
            has_light_token_config: self.parsed.infra_fields.light_token_config.is_some(),
            has_light_token_rent_sponsor: self
                .parsed
                .infra_fields
                .light_token_rent_sponsor
                .is_some(),
            has_light_token_cpi_authority: self
                .parsed
                .infra_fields
                .light_token_cpi_authority
                .is_some(),
            has_instruction_args: self
                .parsed
                .instruction_args
                .as_ref()
                .map(|args| !args.is_empty())
                .unwrap_or(false),
            has_direct_proof_arg: self.parsed.direct_proof_arg.is_some(),
            total_account_count: self.parsed.pda_fields.len()
                + self.parsed.mint_fields.len()
                + self.parsed.token_fields.len()
                + self.parsed.ata_fields.len(),
        };
        validate_struct(&ctx)
    }

    /// Query: any #[light_account(init)] PDA fields?
    pub fn has_pdas(&self) -> bool {
        !self.parsed.pda_fields.is_empty()
    }

    /// Query: any #[light_account(init, mint, ...)] fields?
    pub fn has_mints(&self) -> bool {
        !self.parsed.mint_fields.is_empty()
    }

    /// Query: any #[light_account(..., token, ...)] fields (init or mark-only)?
    pub fn has_token_accounts(&self) -> bool {
        !self.parsed.token_fields.is_empty()
    }

    /// Query: any #[light_account(init, token, ...)] fields specifically?
    /// Used for validation - only init mode requires token infrastructure.
    pub fn has_token_accounts_with_init(&self) -> bool {
        self.parsed.token_fields.iter().any(|f| f.has_init)
    }

    /// Query: any #[light_account(..., associated_token, ...)] fields (init or mark-only)?
    pub fn has_atas(&self) -> bool {
        !self.parsed.ata_fields.is_empty()
    }

    /// Query: any #[light_account(init, associated_token, ...)] fields specifically?
    /// Used for validation - only init mode requires token infrastructure.
    pub fn has_atas_with_init(&self) -> bool {
        self.parsed.ata_fields.iter().any(|f| f.has_init)
    }

    /// Query: #[instruction(...)] present?
    pub fn has_instruction_args(&self) -> bool {
        self.parsed.instruction_args.is_some()
    }

    /// Generate no-op trait impls (for backwards compatibility).
    pub fn generate_noop_impls(&self) -> Result<TokenStream, syn::Error> {
        let struct_name = &self.parsed.struct_name;
        let (impl_generics, ty_generics, where_clause) = self.parsed.generics.split_for_impl();

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics light_account::LightPreInit<light_account::AccountInfo<'info>, ()> for #struct_name #ty_generics #where_clause {
                fn light_pre_init(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    _params: &(),
                ) -> std::result::Result<bool, light_sdk_types::error::LightSdkTypesError> {
                    Ok(false)
                }
            }

            #[automatically_derived]
            impl #impl_generics light_account::LightFinalize<light_account::AccountInfo<'info>, ()> for #struct_name #ty_generics #where_clause {
                fn light_finalize(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    _params: &(),
                    _has_pre_init: bool,
                ) -> std::result::Result<(), light_sdk_types::error::LightSdkTypesError> {
                    Ok(())
                }
            }
        })
    }

    /// Generate LightPreInit body for PDAs + mints:
    /// 1. Write PDAs to CPI context
    /// 2. Invoke CreateMintsCpi with CPI context offset
    ///    After this, Mints are "hot" and usable in instruction body
    pub fn generate_pre_init_pdas_and_mints(&self) -> Result<TokenStream, syn::Error> {
        let body = self.generate_pre_init_pdas_and_mints_body()?;
        Ok(quote! {
            #body
            Ok(true)
        })
    }

    /// Generate LightPreInit body for mints-only (no PDAs):
    /// Invoke CreateMintsCpi with decompress directly
    /// After this, Mints are "hot" and usable in instruction body
    pub fn generate_pre_init_mints_only(&self) -> Result<TokenStream, syn::Error> {
        let body = self.generate_pre_init_mints_only_body()?;
        Ok(quote! {
            #body
            Ok(true)
        })
    }

    /// Generate LightPreInit body for PDAs only (no mints)
    /// After this, compressed addresses are registered
    pub fn generate_pre_init_pdas_only(&self) -> Result<TokenStream, syn::Error> {
        let body = self.generate_pre_init_pdas_only_body()?;
        Ok(quote! {
            #body
            Ok(true)
        })
    }

    /// Generate unified pre_init body for ALL account types.
    ///
    /// This method handles all combinations of:
    /// - PDAs (compressed accounts)
    /// - Mints (compressed mints)
    /// - Token accounts (vaults)
    /// - ATAs (associated token accounts)
    ///
    /// ALL account creation happens here so accounts are available during
    /// the instruction handler for transfers, minting, etc.
    pub fn generate_pre_init_all(&self) -> Result<TokenStream, syn::Error> {
        let has_pdas = self.has_pdas();
        let has_mints = self.has_mints();

        // Generate token/ATA creation code (if any)
        let token_creation = TokenAccountsBuilder::new(
            &self.parsed.token_fields,
            &self.parsed.ata_fields,
            &self.infra,
        )
        .generate_pre_init_token_creation();

        // Handle different combinations
        match (has_pdas, has_mints, token_creation.is_some()) {
            // PDAs + Mints + Tokens/ATAs
            (true, true, true) => {
                let pda_mint_body = self.generate_pre_init_pdas_and_mints_body()?;
                let token_body = token_creation.unwrap();
                Ok(quote! {
                    #pda_mint_body
                    #token_body
                    Ok(true)
                })
            }
            // PDAs + Mints (no tokens)
            (true, true, false) => self.generate_pre_init_pdas_and_mints(),
            // PDAs + Tokens/ATAs (no mints)
            (true, false, true) => {
                let pda_body = self.generate_pre_init_pdas_only_body()?;
                let token_body = token_creation.unwrap();
                Ok(quote! {
                    #pda_body
                    #token_body
                    Ok(true)
                })
            }
            // PDAs only
            (true, false, false) => self.generate_pre_init_pdas_only(),
            // Mints + Tokens/ATAs (no PDAs)
            (false, true, true) => {
                let mint_body = self.generate_pre_init_mints_only_body()?;
                let token_body = token_creation.unwrap();
                Ok(quote! {
                    #mint_body
                    #token_body
                    Ok(true)
                })
            }
            // Mints only
            (false, true, false) => self.generate_pre_init_mints_only(),
            // Tokens/ATAs only (no PDAs, no mints)
            (false, false, true) => {
                let token_body = token_creation.unwrap();
                Ok(quote! {
                    #token_body
                    Ok(true)
                })
            }
            // Nothing to do
            (false, false, false) => Ok(quote! { Ok(false) }),
        }
    }

    /// Generate PDAs + mints body WITHOUT the Ok(true) return.
    fn generate_pre_init_pdas_and_mints_body(&self) -> Result<TokenStream, syn::Error> {
        let compress_blocks = generate_pda_compress_blocks(&self.parsed.pda_fields);
        let rent_reimbursement =
            generate_rent_reimbursement_block(&self.parsed.pda_fields, &self.infra);
        let pda_count = self.parsed.pda_fields.len();
        let rentfree_count = pda_count as u8;

        // Get proof access expression (direct arg or nested in params)
        let proof_access = self.get_proof_access()?;

        let first_pda_output_tree = self.parsed.pda_fields[0]
            .output_tree
            .as_ref()
            .expect("output_tree required for derive macro");

        let mints = &self.parsed.mint_fields;
        let mint_invocation = LightMintsBuilder::new(mints, &proof_access, &self.infra)
            .with_pda_context(pda_count, quote! { #first_pda_output_tree })
            .generate_invocation();

        let fee_payer = &self.infra.fee_payer;
        let compression_config = &self.infra.compression_config;

        Ok(quote! {
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new_with_config(
                &self.#fee_payer,
                _remaining,
                light_sdk::cpi::CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER),
            );
            let compression_config_data = light_account::LightConfig::load_checked(
                &self.#compression_config,
                &crate::ID,
            )?;

            let mut all_new_address_params = Vec::with_capacity(#rentfree_count as usize);
            let mut all_compressed_infos = Vec::with_capacity(#rentfree_count as usize);
            #(#compress_blocks)*

            // Reimburse fee payer for rent paid during PDA creation
            #rent_reimbursement

            light_account::invoke_write_pdas_to_cpi_context(
                crate::LIGHT_CPI_SIGNER,
                #proof_access.proof.clone(),
                &all_new_address_params,
                &all_compressed_infos,
                &cpi_accounts,
            )?;

            #mint_invocation
        })
    }

    /// Generate PDAs-only body WITHOUT the Ok(true) return.
    fn generate_pre_init_pdas_only_body(&self) -> Result<TokenStream, syn::Error> {
        let compress_blocks = generate_pda_compress_blocks(&self.parsed.pda_fields);
        let rent_reimbursement =
            generate_rent_reimbursement_block(&self.parsed.pda_fields, &self.infra);
        let rentfree_count = self.parsed.pda_fields.len() as u8;

        // Get proof access expression (direct arg or nested in params)
        let proof_access = self.get_proof_access()?;

        let fee_payer = &self.infra.fee_payer;
        let compression_config = &self.infra.compression_config;

        Ok(quote! {
            use light_sdk::cpi::{LightCpiInstruction, InvokeLightSystemProgram};

            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(
                &self.#fee_payer,
                _remaining,
                crate::LIGHT_CPI_SIGNER,
            );
            let compression_config_data = light_account::LightConfig::load_checked(
                &self.#compression_config,
                &crate::ID,
            )?;

            let mut all_new_address_params = Vec::with_capacity(#rentfree_count as usize);
            let mut all_compressed_infos = Vec::with_capacity(#rentfree_count as usize);
            #(#compress_blocks)*

            // Reimburse fee payer for rent paid during PDA creation
            #rent_reimbursement

            light_sdk::cpi::v2::LightSystemProgramCpi::new_cpi(
                crate::LIGHT_CPI_SIGNER,
                #proof_access.proof.clone(),
            )
                .with_new_addresses(&all_new_address_params)
                .with_account_infos(&all_compressed_infos)
                .invoke(cpi_accounts)?;
        })
    }

    /// Generate mints-only body WITHOUT the Ok(true) return.
    fn generate_pre_init_mints_only_body(&self) -> Result<TokenStream, syn::Error> {
        // Get proof access expression (direct arg or nested in params)
        let proof_access = self.get_proof_access()?;

        let mints = &self.parsed.mint_fields;
        let mint_invocation =
            LightMintsBuilder::new(mints, &proof_access, &self.infra).generate_invocation();

        let fee_payer = &self.infra.fee_payer;

        Ok(quote! {
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new_with_config(
                &self.#fee_payer,
                _remaining,
                light_sdk::cpi::CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER),
            );

            #mint_invocation
        })
    }

    /// Generate LightPreInit trait implementation.
    pub fn generate_pre_init_impl(&self, body: TokenStream) -> Result<TokenStream, syn::Error> {
        let struct_name = &self.parsed.struct_name;
        let (impl_generics, ty_generics, where_clause) = self.parsed.generics.split_for_impl();

        let first_arg = self.get_first_instruction_arg()?;

        let params_type = &first_arg.ty;
        let params_ident = &first_arg.name;

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics light_account::LightPreInit<light_account::AccountInfo<'info>, #params_type> for #struct_name #ty_generics #where_clause {
                fn light_pre_init(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    #params_ident: &#params_type,
                ) -> std::result::Result<bool, light_sdk_types::error::LightSdkTypesError> {
                    use anchor_lang::ToAccountInfo;
                    #body
                }
            }
        })
    }

    /// Generate LightFinalize trait implementation.
    pub fn generate_finalize_impl(&self, body: TokenStream) -> Result<TokenStream, syn::Error> {
        let struct_name = &self.parsed.struct_name;
        let (impl_generics, ty_generics, where_clause) = self.parsed.generics.split_for_impl();

        let first_arg = self.get_first_instruction_arg()?;

        let params_type = &first_arg.ty;
        let params_ident = &first_arg.name;

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics light_account::LightFinalize<light_account::AccountInfo<'info>, #params_type> for #struct_name #ty_generics #where_clause {
                fn light_finalize(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    #params_ident: &#params_type,
                    _has_pre_init: bool,
                ) -> std::result::Result<(), light_sdk_types::error::LightSdkTypesError> {
                    use anchor_lang::ToAccountInfo;
                    #body
                }
            }
        })
    }
}
