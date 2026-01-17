//! Builder for RentFree derive macro code generation.
//!
//! Encapsulates parsed struct data and resolved infrastructure fields,
//! providing methods for validation, querying, and code generation.

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use super::{
    light_mint::{InfraRefs, LightMintBuilder},
    parse::ParsedRentFreeStruct,
    pda::generate_pda_compress_blocks,
};

/// Builder for RentFree derive macro code generation.
///
/// Encapsulates parsed struct data and resolved infrastructure fields,
/// providing methods for validation, querying, and code generation.
pub(super) struct RentFreeBuilder {
    parsed: ParsedRentFreeStruct,
    infra: InfraRefs,
}

impl RentFreeBuilder {
    /// Parse a DeriveInput and construct the builder.
    pub fn parse(input: &DeriveInput) -> Result<Self, syn::Error> {
        let parsed = super::parse::parse_rentfree_struct(input)?;
        let infra = InfraRefs::from_parsed(&parsed.infra_fields);
        Ok(Self { parsed, infra })
    }

    /// Validate constraints (e.g., account count < 255).
    pub fn validate(&self) -> Result<(), syn::Error> {
        let total = self.parsed.rentfree_fields.len() + self.parsed.light_mint_fields.len();
        if total > 255 {
            return Err(syn::Error::new_spanned(
                &self.parsed.struct_name,
                format!(
                    "Too many compression fields ({} PDAs + {} mints = {} total, maximum 255). \
                     Light Protocol uses u8 for account indices.",
                    self.parsed.rentfree_fields.len(),
                    self.parsed.light_mint_fields.len(),
                    total
                ),
            ));
        }
        Ok(())
    }

    /// Query: any #[rentfree] fields?
    pub fn has_pdas(&self) -> bool {
        !self.parsed.rentfree_fields.is_empty()
    }

    /// Query: any #[light_mint] fields?
    pub fn has_mints(&self) -> bool {
        !self.parsed.light_mint_fields.is_empty()
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
            impl #impl_generics light_sdk::compressible::LightPreInit<'info, ()> for #struct_name #ty_generics #where_clause {
                fn light_pre_init(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    _params: &(),
                ) -> std::result::Result<bool, light_sdk::error::LightSdkError> {
                    Ok(false)
                }
            }

            #[automatically_derived]
            impl #impl_generics light_sdk::compressible::LightFinalize<'info, ()> for #struct_name #ty_generics #where_clause {
                fn light_finalize(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    _params: &(),
                    _has_pre_init: bool,
                ) -> std::result::Result<(), light_sdk::error::LightSdkError> {
                    Ok(())
                }
            }
        })
    }

    /// Generate LightPreInit body for PDAs + mints:
    /// 1. Write PDAs to CPI context
    /// 2. Invoke mint_action with decompress + CPI context
    ///    After this, Mint is "hot" and usable in instruction body
    pub fn generate_pre_init_pdas_and_mints(&self) -> TokenStream {
        let (compress_blocks, new_addr_idents) =
            generate_pda_compress_blocks(&self.parsed.rentfree_fields);
        let rentfree_count = self.parsed.rentfree_fields.len() as u8;
        let pda_count = self.parsed.rentfree_fields.len();

        // Get instruction param ident
        let params_ident = &self
            .parsed
            .instruction_args
            .as_ref()
            .unwrap()
            .first()
            .unwrap()
            .name;

        // Get the first PDA's output tree index (for the state tree output queue)
        let first_pda_output_tree = &self.parsed.rentfree_fields[0].output_tree;

        // TODO(diff-pr): Support multiple #[light_mint] fields by looping here.
        // Each mint would get assigned_account_index = pda_count + mint_index.
        // Also add support for #[rentfree_token] fields for token ATAs.
        let mint = &self.parsed.light_mint_fields[0];

        // assigned_account_index for mint is after PDAs
        let mint_assigned_index = pda_count as u8;

        // Generate mint action invocation with CPI context
        let mint_invocation = LightMintBuilder::new(mint, params_ident, &self.infra)
            .with_cpi_context(quote! { #first_pda_output_tree }, mint_assigned_index)
            .generate_invocation();

        // Infrastructure field references for quote! interpolation
        let fee_payer = &self.infra.fee_payer;
        let compression_config = &self.infra.compression_config;

        quote! {
            // Build CPI accounts WITH CPI context for batching
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new_with_config(
                &self.#fee_payer,
                _remaining,
                light_sdk_types::cpi_accounts::CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER),
            );

            // Load compression config
            let compression_config_data = light_sdk::compressible::CompressibleConfig::load_checked(
                &self.#compression_config,
                &crate::ID
            )?;

            // Collect compressed infos for all rentfree PDA accounts
            let mut all_compressed_infos = Vec::with_capacity(#rentfree_count as usize);
            #(#compress_blocks)*

            // Step 1: Write PDAs to CPI context
            let cpi_context_account = cpi_accounts.cpi_context()?;
            let cpi_context_accounts = light_sdk_types::cpi_context_write::CpiContextWriteAccounts {
                fee_payer: cpi_accounts.fee_payer(),
                authority: cpi_accounts.authority()?,
                cpi_context: cpi_context_account,
                cpi_signer: crate::LIGHT_CPI_SIGNER,
            };

            use light_sdk::cpi::{InvokeLightSystemProgram, LightCpiInstruction};
            light_sdk::cpi::v2::LightSystemProgramCpi::new_cpi(
                crate::LIGHT_CPI_SIGNER,
                #params_ident.create_accounts_proof.proof.clone()
            )
                .with_new_addresses(&[#(#new_addr_idents),*])
                .with_account_infos(&all_compressed_infos)
                .write_to_cpi_context_first()
                .invoke_write_to_cpi_context_first(cpi_context_accounts)?;

            // Step 2: Build and invoke mint_action with decompress + CPI context
            #mint_invocation

            Ok(true)
        }
    }

    /// Generate LightPreInit body for mints-only (no PDAs):
    /// Invoke mint_action with decompress directly
    /// After this, CMint is "hot" and usable in instruction body
    pub fn generate_pre_init_mints_only(&self) -> TokenStream {
        // Get instruction param ident
        let params_ident = &self
            .parsed
            .instruction_args
            .as_ref()
            .unwrap()
            .first()
            .unwrap()
            .name;

        // TODO(diff-pr): Support multiple #[light_mint] fields by looping here.
        // Each mint would get assigned_account_index = mint_index.
        // Also add support for #[rentfree_token] fields for token ATAs.
        let mint = &self.parsed.light_mint_fields[0];

        // Generate mint action invocation without CPI context
        let mint_invocation =
            LightMintBuilder::new(mint, params_ident, &self.infra).generate_invocation();

        // Infrastructure field reference for quote! interpolation
        let fee_payer = &self.infra.fee_payer;

        quote! {
            // Build CPI accounts (no CPI context needed for mints-only)
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(
                &self.#fee_payer,
                _remaining,
                crate::LIGHT_CPI_SIGNER,
            );

            // Build and invoke mint_action with decompress
            #mint_invocation

            Ok(true)
        }
    }

    /// Generate LightPreInit body for PDAs only (no mints)
    /// After this, compressed addresses are registered
    pub fn generate_pre_init_pdas_only(&self) -> TokenStream {
        let (compress_blocks, new_addr_idents) =
            generate_pda_compress_blocks(&self.parsed.rentfree_fields);
        let rentfree_count = self.parsed.rentfree_fields.len() as u8;

        // Get instruction param ident
        let params_ident = &self
            .parsed
            .instruction_args
            .as_ref()
            .unwrap()
            .first()
            .unwrap()
            .name;

        // Infra field references
        let fee_payer = &self.infra.fee_payer;
        let compression_config = &self.infra.compression_config;

        quote! {
            // Build CPI accounts (no CPI context needed for PDAs-only)
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(
                &self.#fee_payer,
                _remaining,
                crate::LIGHT_CPI_SIGNER,
            );

            // Load compression config
            let compression_config_data = light_sdk::compressible::CompressibleConfig::load_checked(
                &self.#compression_config,
                &crate::ID
            )?;

            // Collect compressed infos for all rentfree accounts
            let mut all_compressed_infos = Vec::with_capacity(#rentfree_count as usize);
            #(#compress_blocks)*

            // Execute Light System Program CPI directly with proof
            use light_sdk::cpi::{InvokeLightSystemProgram, LightCpiInstruction};
            light_sdk::cpi::v2::LightSystemProgramCpi::new_cpi(
                crate::LIGHT_CPI_SIGNER,
                #params_ident.create_accounts_proof.proof.clone()
            )
                .with_new_addresses(&[#(#new_addr_idents),*])
                .with_account_infos(&all_compressed_infos)
                .invoke(cpi_accounts)?;

            Ok(true)
        }
    }

    /// Generate LightPreInit trait implementation.
    pub fn generate_pre_init_impl(&self, body: TokenStream) -> TokenStream {
        let struct_name = &self.parsed.struct_name;
        let (impl_generics, ty_generics, where_clause) = self.parsed.generics.split_for_impl();

        let first_arg = self
            .parsed
            .instruction_args
            .as_ref()
            .and_then(|args| args.first())
            .unwrap();

        let params_type = &first_arg.ty;
        let params_ident = &first_arg.name;

        quote! {
            #[automatically_derived]
            impl #impl_generics light_sdk::compressible::LightPreInit<'info, #params_type> for #struct_name #ty_generics #where_clause {
                fn light_pre_init(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    #params_ident: &#params_type,
                ) -> std::result::Result<bool, light_sdk::error::LightSdkError> {
                    use anchor_lang::ToAccountInfo;
                    #body
                }
            }
        }
    }

    /// Generate LightFinalize trait implementation.
    pub fn generate_finalize_impl(&self, body: TokenStream) -> TokenStream {
        let struct_name = &self.parsed.struct_name;
        let (impl_generics, ty_generics, where_clause) = self.parsed.generics.split_for_impl();

        let first_arg = self
            .parsed
            .instruction_args
            .as_ref()
            .and_then(|args| args.first())
            .unwrap();

        let params_type = &first_arg.ty;
        let params_ident = &first_arg.name;

        quote! {
            #[automatically_derived]
            impl #impl_generics light_sdk::compressible::LightFinalize<'info, #params_type> for #struct_name #ty_generics #where_clause {
                fn light_finalize(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    #params_ident: &#params_type,
                    _has_pre_init: bool,
                ) -> std::result::Result<(), light_sdk::error::LightSdkError> {
                    use anchor_lang::ToAccountInfo;
                    #body
                }
            }
        }
    }
}
