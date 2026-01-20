//! Builder for RentFree derive macro code generation.
//!
//! Encapsulates parsed struct data and resolved infrastructure fields,
//! providing methods for validation, querying, and code generation.

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use super::{
    mint::{InfraRefs, LightMintsBuilder},
    parse::{InfraFieldType, ParsedLightAccountsStruct},
    pda::generate_pda_compress_blocks,
    token::TokenAccountsBuilder,
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

    /// Validate constraints (e.g., account count < 255).
    pub fn validate(&self) -> Result<(), syn::Error> {
        let total = self.parsed.rentfree_fields.len()
            + self.parsed.light_mint_fields.len()
            + self.parsed.token_account_fields.len()
            + self.parsed.ata_fields.len();
        if total > 255 {
            return Err(syn::Error::new_spanned(
                &self.parsed.struct_name,
                format!(
                    "Too many compression fields ({} PDAs + {} mints + {} tokens + {} ATAs = {} total, maximum 255). \
                     Light Protocol uses u8 for account indices.",
                    self.parsed.rentfree_fields.len(),
                    self.parsed.light_mint_fields.len(),
                    self.parsed.token_account_fields.len(),
                    self.parsed.ata_fields.len(),
                    total
                ),
            ));
        }

        // Validate infrastructure fields are present
        self.validate_infra_fields()?;

        Ok(())
    }

    /// Validate that required infrastructure fields are present.
    fn validate_infra_fields(&self) -> Result<(), syn::Error> {
        let has_pdas = self.has_pdas();
        let has_mints = self.has_mints();
        let has_token_accounts = self.has_token_accounts();
        let has_atas = self.has_atas();

        // Skip validation if no light_account fields
        if !has_pdas && !has_mints && !has_token_accounts && !has_atas {
            return Ok(());
        }

        let mut missing = Vec::new();

        // fee_payer is always required
        if self.parsed.infra_fields.fee_payer.is_none() {
            missing.push(InfraFieldType::FeePayer);
        }

        // PDAs require compression_config
        if has_pdas && self.parsed.infra_fields.compression_config.is_none() {
            missing.push(InfraFieldType::CompressionConfig);
        }

        // Mints, token accounts, and ATAs require light_token infrastructure
        let needs_token_infra = has_mints || has_token_accounts || has_atas;
        if needs_token_infra {
            if self.parsed.infra_fields.light_token_config.is_none() {
                missing.push(InfraFieldType::LightTokenConfig);
            }
            if self.parsed.infra_fields.light_token_rent_sponsor.is_none() {
                missing.push(InfraFieldType::LightTokenRentSponsor);
            }
            // CPI authority is required for mints and token accounts (PDA-based signing)
            if (has_mints || has_token_accounts)
                && self.parsed.infra_fields.light_token_cpi_authority.is_none()
            {
                missing.push(InfraFieldType::LightTokenCpiAuthority);
            }
        }

        if !missing.is_empty() {
            let mut types = Vec::new();
            if has_pdas {
                types.push("PDA");
            }
            if has_mints {
                types.push("mint");
            }
            if has_token_accounts {
                types.push("token account");
            }
            if has_atas {
                types.push("ATA");
            }
            let context = types.join(", ");

            let mut msg = format!(
                "#[derive(LightAccounts)] with {} fields requires the following infrastructure fields:\n",
                context
            );

            for field_type in &missing {
                msg.push_str(&format!(
                    "\n  - {} (add one of: {})",
                    field_type.description(),
                    field_type.accepted_names().join(", ")
                ));
            }

            return Err(syn::Error::new_spanned(&self.parsed.struct_name, msg));
        }

        Ok(())
    }

    /// Query: any #[light_account(init)] PDA fields?
    pub fn has_pdas(&self) -> bool {
        !self.parsed.rentfree_fields.is_empty()
    }

    /// Query: any #[light_account(init, mint, ...)] fields?
    pub fn has_mints(&self) -> bool {
        !self.parsed.light_mint_fields.is_empty()
    }

    /// Query: any #[light_account(init, token, ...)] fields?
    pub fn has_token_accounts(&self) -> bool {
        !self.parsed.token_account_fields.is_empty()
    }

    /// Query: any #[light_account(init, associated_token, ...)] fields?
    pub fn has_atas(&self) -> bool {
        !self.parsed.ata_fields.is_empty()
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
            impl #impl_generics light_sdk::interface::LightPreInit<'info, ()> for #struct_name #ty_generics #where_clause {
                fn light_pre_init(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    _params: &(),
                ) -> std::result::Result<bool, light_sdk::error::LightSdkError> {
                    Ok(false)
                }
            }

            #[automatically_derived]
            impl #impl_generics light_sdk::interface::LightFinalize<'info, ()> for #struct_name #ty_generics #where_clause {
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
    /// 2. Invoke CreateMintsCpi with CPI context offset
    ///    After this, Mints are "hot" and usable in instruction body
    pub fn generate_pre_init_pdas_and_mints(&self) -> Result<TokenStream, syn::Error> {
        let (compress_blocks, new_addr_idents) =
            generate_pda_compress_blocks(&self.parsed.rentfree_fields);
        let rentfree_count = self.parsed.rentfree_fields.len() as u8;
        let pda_count = self.parsed.rentfree_fields.len();

        // Get instruction param ident
        let first_arg = self.get_first_instruction_arg()?;
        let params_ident = &first_arg.name;

        // Get the first PDA's output tree index (for the state tree output queue)
        let first_pda_output_tree = &self.parsed.rentfree_fields[0].output_tree;

        // Generate CreateMintsCpi invocation for all mints with PDA context offset
        let mints = &self.parsed.light_mint_fields;
        let mint_invocation = LightMintsBuilder::new(mints, params_ident, &self.infra)
            .with_pda_context(pda_count, quote! { #first_pda_output_tree })
            .generate_invocation();

        // Infrastructure field references for quote! interpolation
        let fee_payer = &self.infra.fee_payer;
        let compression_config = &self.infra.compression_config;

        Ok(quote! {
            // Build CPI accounts WITH CPI context for batching
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new_with_config(
                &self.#fee_payer,
                _remaining,
                ::light_sdk::sdk_types::CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER),
            );

            // Load compression config
            let compression_config_data = light_sdk::interface::LightConfig::load_checked(
                &self.#compression_config,
                &crate::ID
            )?;

            // Collect compressed infos for all rentfree PDA accounts
            let mut all_compressed_infos = Vec::with_capacity(#rentfree_count as usize);
            #(#compress_blocks)*

            // Step 1: Write PDAs to CPI context
            let cpi_context_account = cpi_accounts.cpi_context()?;
            let cpi_context_accounts = ::light_sdk::sdk_types::CpiContextWriteAccounts {
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

            // Step 2: Create mints using CreateMintsCpi with CPI context offset
            #mint_invocation

            Ok(true)
        })
    }

    /// Generate LightPreInit body for mints-only (no PDAs):
    /// Invoke CreateMintsCpi with decompress directly
    /// After this, Mints are "hot" and usable in instruction body
    pub fn generate_pre_init_mints_only(&self) -> Result<TokenStream, syn::Error> {
        // Get instruction param ident
        let first_arg = self.get_first_instruction_arg()?;
        let params_ident = &first_arg.name;

        // Generate CreateMintsCpi invocation for all mints (no PDA context)
        let mints = &self.parsed.light_mint_fields;
        let mint_invocation =
            LightMintsBuilder::new(mints, params_ident, &self.infra).generate_invocation();

        // Infrastructure field reference for quote! interpolation
        let fee_payer = &self.infra.fee_payer;

        Ok(quote! {
            // Build CPI accounts with CPI context enabled (mints use CPI context for batching)
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new_with_config(
                &self.#fee_payer,
                _remaining,
                light_sdk::cpi::CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER),
            );

            // Create mints using CreateMintsCpi
            #mint_invocation

            Ok(true)
        })
    }

    /// Generate LightPreInit body for PDAs only (no mints)
    /// After this, compressed addresses are registered
    pub fn generate_pre_init_pdas_only(&self) -> Result<TokenStream, syn::Error> {
        let (compress_blocks, new_addr_idents) =
            generate_pda_compress_blocks(&self.parsed.rentfree_fields);
        let rentfree_count = self.parsed.rentfree_fields.len() as u8;

        // Get instruction param ident
        let first_arg = self.get_first_instruction_arg()?;
        let params_ident = &first_arg.name;

        // Infra field references
        let fee_payer = &self.infra.fee_payer;
        let compression_config = &self.infra.compression_config;

        Ok(quote! {
            // Build CPI accounts (no CPI context needed for PDAs-only)
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(
                &self.#fee_payer,
                _remaining,
                crate::LIGHT_CPI_SIGNER,
            );

            // Load compression config
            let compression_config_data = light_sdk::interface::LightConfig::load_checked(
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
            impl #impl_generics light_sdk::interface::LightPreInit<'info, #params_type> for #struct_name #ty_generics #where_clause {
                fn light_pre_init(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    #params_ident: &#params_type,
                ) -> std::result::Result<bool, light_sdk::error::LightSdkError> {
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
            impl #impl_generics light_sdk::interface::LightFinalize<'info, #params_type> for #struct_name #ty_generics #where_clause {
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
        })
    }

    /// Check if token accounts or ATAs need finalize code generation.
    pub fn needs_token_finalize(&self) -> bool {
        TokenAccountsBuilder::new(
            &self.parsed.token_account_fields,
            &self.parsed.ata_fields,
            &self.infra,
        )
        .needs_finalize()
    }

    /// Generate finalize body for token accounts and ATAs.
    pub fn generate_token_finalize_body(&self) -> TokenStream {
        TokenAccountsBuilder::new(
            &self.parsed.token_account_fields,
            &self.parsed.ata_fields,
            &self.infra,
        )
        .generate_finalize_body()
    }
}
