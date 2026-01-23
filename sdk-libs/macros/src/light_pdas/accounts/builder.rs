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
use crate::utils::to_snake_case;

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

        // Validate CreateAccountsProof is available
        self.validate_create_accounts_proof()?;

        Ok(())
    }

    /// Validate that CreateAccountsProof is available when needed.
    ///
    /// CreateAccountsProof is required when there are any init fields (PDAs, mints).
    /// It can be provided either:
    /// - As a direct argument: `proof: CreateAccountsProof`
    /// - As a field on the first instruction arg: `params.create_accounts_proof`
    fn validate_create_accounts_proof(&self) -> Result<(), syn::Error> {
        let needs_proof = self.has_pdas() || self.has_mints();

        if !needs_proof {
            return Ok(());
        }

        // Check if CreateAccountsProof is available
        let has_direct_proof = self.parsed.direct_proof_arg.is_some();
        let has_instruction_args = self
            .parsed
            .instruction_args
            .as_ref()
            .map(|args| !args.is_empty())
            .unwrap_or(false);

        if !has_direct_proof && !has_instruction_args {
            return Err(syn::Error::new_spanned(
                &self.parsed.struct_name,
                "CreateAccountsProof is required for #[light_account(init)] fields.\n\
                 \n\
                 Provide it either:\n\
                 1. As a direct argument: #[instruction(proof: CreateAccountsProof)]\n\
                 2. As a field on params: #[instruction(params: MyParams)] where MyParams has a `create_accounts_proof: CreateAccountsProof` field",
            ));
        }

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
            &self.parsed.token_account_fields,
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
        let (compress_blocks, new_addr_idents) =
            generate_pda_compress_blocks(&self.parsed.rentfree_fields);
        let rentfree_count = self.parsed.rentfree_fields.len() as u8;
        let pda_count = self.parsed.rentfree_fields.len();

        // Get proof access expression (direct arg or nested in params)
        let proof_access = self.get_proof_access()?;

        let first_pda_output_tree = &self.parsed.rentfree_fields[0].output_tree;

        let mints = &self.parsed.light_mint_fields;
        let mint_invocation = LightMintsBuilder::new(mints, &proof_access, &self.infra)
            .with_pda_context(pda_count, quote! { #first_pda_output_tree })
            .generate_invocation();

        let fee_payer = &self.infra.fee_payer;
        let compression_config = &self.infra.compression_config;

        Ok(quote! {
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new_with_config(
                &self.#fee_payer,
                _remaining,
                ::light_sdk::sdk_types::CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER),
            );

            let compression_config_data = light_sdk::interface::LightConfig::load_checked(
                &self.#compression_config,
                &crate::ID
            )?;

            let mut all_compressed_infos = Vec::with_capacity(#rentfree_count as usize);
            #(#compress_blocks)*

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
                #proof_access.proof.clone()
            )
                .with_new_addresses(&[#(#new_addr_idents),*])
                .with_account_infos(&all_compressed_infos)
                .write_to_cpi_context_first()
                .invoke_write_to_cpi_context_first(cpi_context_accounts)?;

            #mint_invocation
        })
    }

    /// Generate PDAs-only body WITHOUT the Ok(true) return.
    fn generate_pre_init_pdas_only_body(&self) -> Result<TokenStream, syn::Error> {
        let (compress_blocks, new_addr_idents) =
            generate_pda_compress_blocks(&self.parsed.rentfree_fields);
        let rentfree_count = self.parsed.rentfree_fields.len() as u8;

        // Get proof access expression (direct arg or nested in params)
        let proof_access = self.get_proof_access()?;

        let fee_payer = &self.infra.fee_payer;
        let compression_config = &self.infra.compression_config;

        Ok(quote! {
            let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(
                &self.#fee_payer,
                _remaining,
                crate::LIGHT_CPI_SIGNER,
            );

            let compression_config_data = light_sdk::interface::LightConfig::load_checked(
                &self.#compression_config,
                &crate::ID
            )?;

            let mut all_compressed_infos = Vec::with_capacity(#rentfree_count as usize);
            #(#compress_blocks)*

            use light_sdk::cpi::{InvokeLightSystemProgram, LightCpiInstruction};
            light_sdk::cpi::v2::LightSystemProgramCpi::new_cpi(
                crate::LIGHT_CPI_SIGNER,
                #proof_access.proof.clone()
            )
                .with_new_addresses(&[#(#new_addr_idents),*])
                .with_account_infos(&all_compressed_infos)
                .invoke(cpi_accounts)?;
        })
    }

    /// Generate mints-only body WITHOUT the Ok(true) return.
    fn generate_pre_init_mints_only_body(&self) -> Result<TokenStream, syn::Error> {
        // Get proof access expression (direct arg or nested in params)
        let proof_access = self.get_proof_access()?;

        let mints = &self.parsed.light_mint_fields;
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

    /// Query: any field with `LightMint<'info>` type?
    pub fn has_light_mint_type_fields(&self) -> bool {
        self.parsed.has_light_mint_type_fields
    }

    /// Generate Anchor trait implementations for structs with LightMint fields.
    ///
    /// When a struct contains `LightMint<'info>` fields, Anchor's `#[derive(Accounts)]`
    /// fails because `LightMint` is not in Anchor's hardcoded primitive type whitelist.
    /// This method generates the necessary trait implementations manually.
    ///
    /// Generated traits:
    /// - `Accounts<'info, B>` - Account deserialization
    /// - `AccountsExit<'info>` - Account serialization on exit
    /// - `ToAccountInfos<'info>` - Convert to account info list
    /// - `ToAccountMetas` - Convert to account meta list
    /// - `Bumps` - Anchor Bumps trait for Context compatibility
    pub fn generate_anchor_accounts_impl(&self) -> Result<TokenStream, syn::Error> {
        let struct_name = &self.parsed.struct_name;
        let (impl_generics, ty_generics, where_clause) = self.parsed.generics.split_for_impl();
        let fields = &self.parsed.all_fields;

        // Generate field assignments for try_accounts
        let field_assignments: Vec<TokenStream> = fields
            .iter()
            .map(|f| {
                let field_ident = &f.ident;
                let field_ty = &f.ty;
                quote! {
                    let #field_ident: #field_ty = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )?;
                }
            })
            .collect();

        let field_names: Vec<&syn::Ident> = fields.iter().map(|f| &f.ident).collect();

        // Generate exit calls - only for mutable fields
        // Non-mutable fields like Program<'info, System> don't implement AccountsExit
        let exit_calls: Vec<TokenStream> = fields
            .iter()
            .filter(|f| f.is_mut)
            .map(|f| {
                let field_ident = &f.ident;
                quote! {
                    anchor_lang::AccountsExit::exit(&self.#field_ident, program_id)?;
                }
            })
            .collect();

        // Generate to_account_infos calls
        let account_info_calls: Vec<TokenStream> = fields
            .iter()
            .map(|f| {
                let field_ident = &f.ident;
                quote! {
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.#field_ident));
                }
            })
            .collect();

        // Generate to_account_metas calls
        let account_meta_calls: Vec<TokenStream> = fields
            .iter()
            .map(|f| {
                let field_ident = &f.ident;
                quote! {
                    account_metas.extend(anchor_lang::ToAccountMetas::to_account_metas(&self.#field_ident, None));
                }
            })
            .collect();

        let field_count = fields.len();

        // Generate the Bumps struct for Anchor compatibility
        let bumps_struct_name = syn::Ident::new(
            &format!("{}Bumps", struct_name),
            struct_name.span(),
        );

        // Generate client accounts module name (snake_case of struct name)
        let struct_name_str = struct_name.to_string();
        let client_module_name = syn::Ident::new(
            &format!(
                "__client_accounts_{}",
                to_snake_case(&struct_name_str)
            ),
            struct_name.span(),
        );

        // Generate fields for the client accounts struct
        // Each field maps to a Pubkey (accounts are represented by their keys in client code)
        let client_struct_fields: Vec<TokenStream> = fields
            .iter()
            .map(|f| {
                let field_ident = &f.ident;
                quote! {
                    pub #field_ident: anchor_lang::prelude::Pubkey,
                }
            })
            .collect();

        // Generate ToAccountMetas for client struct
        let client_to_account_metas: Vec<TokenStream> = fields
            .iter()
            .map(|f| {
                let field_ident = &f.ident;
                if f.is_mut {
                    if f.is_signer {
                        quote! {
                            account_metas.push(anchor_lang::prelude::AccountMeta::new(self.#field_ident, true));
                        }
                    } else {
                        quote! {
                            account_metas.push(anchor_lang::prelude::AccountMeta::new(self.#field_ident, false));
                        }
                    }
                } else if f.is_signer {
                    quote! {
                        account_metas.push(anchor_lang::prelude::AccountMeta::new_readonly(self.#field_ident, true));
                    }
                } else {
                    quote! {
                        account_metas.push(anchor_lang::prelude::AccountMeta::new_readonly(self.#field_ident, false));
                    }
                }
            })
            .collect();

        Ok(quote! {
            /// Auto-generated client accounts module for Anchor compatibility.
            /// This module is required by Anchor's #[program] macro.
            pub mod #client_module_name {
                use super::*;

                /// Client-side representation of the accounts struct.
                /// Used for building instructions in client code.
                #[derive(Clone)]
                pub struct #struct_name {
                    #(#client_struct_fields)*
                }

                impl anchor_lang::ToAccountMetas for #struct_name {
                    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
                        let mut account_metas = Vec::with_capacity(#field_count);
                        #(#client_to_account_metas)*
                        account_metas
                    }
                }
            }

            /// Auto-generated Bumps struct for Anchor compatibility.
            #[derive(Default, Debug, Clone)]
            pub struct #bumps_struct_name {
                // Empty - bumps are handled separately for LightMint fields
            }

            impl #bumps_struct_name {
                /// Get a bump by name (returns None for LightMint-based structs).
                pub fn get(&self, _name: &str) -> Option<u8> {
                    None
                }
            }

            /// Anchor Bumps trait implementation for Context compatibility.
            #[automatically_derived]
            impl #impl_generics anchor_lang::Bumps for #struct_name #ty_generics #where_clause {
                type Bumps = #bumps_struct_name;
            }

            /// IDL generation method required by Anchor's #[program] macro.
            /// This generates a minimal IDL representation for LightMint-based structs.
            impl #impl_generics #struct_name #ty_generics #where_clause {
                pub fn __anchor_private_gen_idl_accounts(
                    _accounts: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlAccount>,
                    _types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
                ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
                    // Generate minimal account info for each field
                    vec![
                        #(
                            anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                                anchor_lang::idl::types::IdlInstructionAccount {
                                    name: stringify!(#field_names).into(),
                                    docs: vec![],
                                    writable: false, // Could be enhanced to check is_mut
                                    signer: false,   // Could be enhanced to check is_signer
                                    optional: false,
                                    address: None,
                                    pda: None,
                                    relations: vec![],
                                }
                            )
                        ),*
                    ]
                }
            }

            #[automatically_derived]
            impl #impl_generics anchor_lang::Accounts<'info, #bumps_struct_name> for #struct_name #ty_generics #where_clause {
                fn try_accounts(
                    __program_id: &anchor_lang::prelude::Pubkey,
                    __accounts: &mut &'info [anchor_lang::prelude::AccountInfo<'info>],
                    __ix_data: &[u8],
                    __bumps: &mut #bumps_struct_name,
                    __reallocs: &mut std::collections::BTreeSet<anchor_lang::prelude::Pubkey>,
                ) -> anchor_lang::Result<Self> {
                    #(#field_assignments)*

                    Ok(Self {
                        #(#field_names),*
                    })
                }
            }

            #[automatically_derived]
            impl #impl_generics anchor_lang::AccountsExit<'info> for #struct_name #ty_generics #where_clause {
                fn exit(&self, program_id: &anchor_lang::prelude::Pubkey) -> anchor_lang::Result<()> {
                    #(#exit_calls)*
                    Ok(())
                }
            }

            #[automatically_derived]
            impl #impl_generics anchor_lang::ToAccountInfos<'info> for #struct_name #ty_generics #where_clause {
                fn to_account_infos(&self) -> Vec<anchor_lang::prelude::AccountInfo<'info>> {
                    let mut account_infos = Vec::with_capacity(#field_count);
                    #(#account_info_calls)*
                    account_infos
                }
            }

            #[automatically_derived]
            impl #impl_generics anchor_lang::ToAccountMetas for #struct_name #ty_generics #where_clause {
                fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
                    let mut account_metas = Vec::with_capacity(#field_count);
                    #(#account_meta_calls)*
                    account_metas
                }
            }
        })
    }
}
