//! Builder for RentFree derive macro code generation.
//!
//! Encapsulates parsed struct data and resolved infrastructure fields,
//! providing methods for validation, querying, and code generation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

use super::{
    light_account::{AtaField, TokenAccountField},
    mint::{InfraRefs, LightMintField},
    parse::{ParsedLightAccountsStruct, ParsedPdaField},
    pda::{generate_pda_compress_blocks, generate_rent_reimbursement_block},
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
                ) -> std::result::Result<bool, light_account::LightSdkTypesError> {
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
                ) -> std::result::Result<(), light_account::LightSdkTypesError> {
                    Ok(())
                }
            }
        })
    }

    /// Generate LightPreInit body for PDAs only (no mints/tokens/ATAs).
    ///
    /// This path does NOT require the `token` feature on `light-account`.
    fn generate_pre_init_pdas_only(&self) -> Result<TokenStream, syn::Error> {
        let compress_blocks = generate_pda_compress_blocks(&self.parsed.pda_fields);
        let rent_reimbursement =
            generate_rent_reimbursement_block(&self.parsed.pda_fields, &self.infra);
        let rentfree_count = self.parsed.pda_fields.len() as u8;

        let proof_access = self.get_proof_access()?;
        let fee_payer = &self.infra.fee_payer;
        let compression_config = &self.infra.compression_config;

        Ok(quote! {
            use light_account::InvokeLightSystemProgram;

            let cpi_accounts = light_account::CpiAccounts::new(
                &self.#fee_payer,
                _remaining,
                crate::LIGHT_CPI_SIGNER,
            );
            let compression_config_data = light_account::LightConfig::load_checked(
                &self.#compression_config,
                &crate::LIGHT_CPI_SIGNER.program_id,
            )?;

            let mut all_new_address_params = Vec::with_capacity(#rentfree_count as usize);
            let mut all_compressed_infos = Vec::with_capacity(#rentfree_count as usize);
            #(#compress_blocks)*

            // Reimburse fee payer for rent paid during PDA creation
            #rent_reimbursement

            let instruction_data = light_compressed_account::instruction_data::with_account_info::InstructionDataInvokeCpiWithAccountInfo {
                mode: 1,
                bump: crate::LIGHT_CPI_SIGNER.bump,
                invoking_program_id: crate::LIGHT_CPI_SIGNER.program_id.into(),
                compress_or_decompress_lamports: 0,
                is_compress: false,
                with_cpi_context: false,
                with_transaction_hash: false,
                cpi_context: light_compressed_account::instruction_data::cpi_context::CompressedCpiContext::default(),
                proof: #proof_access.proof.clone().0,
                new_address_params: all_new_address_params,
                account_infos: all_compressed_infos,
                read_only_addresses: vec![],
                read_only_accounts: vec![],
            };
            instruction_data.invoke(cpi_accounts)?;

            Ok(true)
        })
    }

    /// Generate unified pre_init body for ALL account types.
    ///
    /// Uses `create_accounts()` for any combination involving mints/tokens/ATAs.
    /// Falls back to direct PDA codegen for PDA-only (no `token` feature needed).
    pub fn generate_pre_init_all(&self) -> Result<TokenStream, syn::Error> {
        let has_pdas = self.has_pdas();
        let has_mints = self.has_mints();
        let has_tokens_with_init = self.has_token_accounts_with_init();
        let has_atas_with_init = self.has_atas_with_init();
        let has_token_anything = has_mints || has_tokens_with_init || has_atas_with_init;

        match (has_pdas, has_token_anything) {
            (false, false) => Ok(quote! { Ok(false) }),
            (true, false) => self.generate_pre_init_pdas_only(),
            (_, true) => self.generate_pre_init_with_create_accounts(),
        }
    }

    /// Generate pre_init body using `create_accounts()` SDK function.
    ///
    /// Handles all combinations involving mints, tokens, or ATAs (with or without PDAs).
    /// Requires the `token` feature on `light-account`.
    fn generate_pre_init_with_create_accounts(&self) -> Result<TokenStream, syn::Error> {
        let proof_access = self.get_proof_access()?;
        let has_pdas = self.has_pdas();
        let has_mints = self.has_mints();
        let has_tokens_with_init = self.has_token_accounts_with_init();
        let has_atas_with_init = self.has_atas_with_init();

        let pda_count = self.parsed.pda_fields.len();
        let mint_count = self.parsed.mint_fields.len();
        let token_init_fields: Vec<&TokenAccountField> = self
            .parsed
            .token_fields
            .iter()
            .filter(|f| f.has_init)
            .collect();
        let ata_init_fields: Vec<&AtaField> = self
            .parsed
            .ata_fields
            .iter()
            .filter(|f| f.has_init)
            .collect();
        let token_init_count = token_init_fields.len();
        let ata_init_count = ata_init_fields.len();

        // --- PDA account info bindings and PdaInitParam elements ---
        let pda_info_bindings: Vec<TokenStream> = self
            .parsed
            .pda_fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let field_name = &field.field_name;
                let info_ident = format_ident!("__pda_info_{}", i);
                quote! { let #info_ident = self.#field_name.to_account_info(); }
            })
            .collect();

        let pda_init_params: Vec<TokenStream> = (0..pda_count)
            .map(|i| {
                let info_ident = format_ident!("__pda_info_{}", i);
                quote! { light_account::PdaInitParam { account: &#info_ident } }
            })
            .collect();

        // --- PDA setup closure body ---
        let pda_setup_body = generate_pda_setup_closure_body(&self.parsed.pda_fields);

        // --- Mint params and CreateMintsInput ---
        let (mint_bindings, mint_input_expr) = if has_mints {
            generate_mint_input(&self.parsed.mint_fields)?
        } else {
            (quote! {}, quote! { None })
        };

        // --- Token init params ---
        let (token_bindings, token_init_params) = generate_token_init_params(&token_init_fields);

        // --- ATA init params ---
        let (ata_bindings, ata_init_params) = generate_ata_init_params(&ata_init_fields);

        // --- Infrastructure account info bindings ---
        let fee_payer = &self.infra.fee_payer;
        let mut infra_bindings =
            vec![quote! { let __fee_payer_info = self.#fee_payer.to_account_info(); }];

        let compression_config_opt = if has_pdas {
            let cc = &self.infra.compression_config;
            infra_bindings
                .push(quote! { let __compression_config_info = self.#cc.to_account_info(); });
            quote! { Some(&__compression_config_info) }
        } else {
            quote! { None }
        };

        let has_token_infra = has_mints || has_tokens_with_init || has_atas_with_init;
        let compressible_config_opt = if has_token_infra {
            let ltc = &self.infra.light_token_config;
            infra_bindings
                .push(quote! { let __compressible_config_info = self.#ltc.to_account_info(); });
            quote! { Some(&__compressible_config_info) }
        } else {
            quote! { None }
        };

        let rent_sponsor_opt = if has_token_infra {
            let ltrs = &self.infra.light_token_rent_sponsor;
            infra_bindings.push(quote! { let __rent_sponsor_info = self.#ltrs.to_account_info(); });
            quote! { Some(&__rent_sponsor_info) }
        } else {
            quote! { None }
        };

        let cpi_authority_opt = if has_mints {
            let ltca = &self.infra.light_token_cpi_authority;
            infra_bindings
                .push(quote! { let __cpi_authority_info = self.#ltca.to_account_info(); });
            quote! { Some(&__cpi_authority_info) }
        } else {
            quote! { None }
        };

        let system_program_opt = if has_tokens_with_init || has_atas_with_init {
            infra_bindings.push(
                quote! { let __system_program_info = self.system_program.to_account_info(); },
            );
            quote! { Some(&__system_program_info) }
        } else {
            quote! { None }
        };

        // --- Reimburse rent for PDAs (Anchor-specific) ---
        let reimburse_block = if has_pdas {
            let compression_config = &self.infra.compression_config;
            let rent_reimbursement =
                generate_rent_reimbursement_block(&self.parsed.pda_fields, &self.infra);
            quote! {
                let compression_config_data = light_account::LightConfig::load_checked(
                    &self.#compression_config,
                    &crate::LIGHT_CPI_SIGNER.program_id,
                )?;
                #rent_reimbursement
            }
        } else {
            quote! {}
        };

        Ok(quote! {
            // Infrastructure account info bindings
            #(#infra_bindings)*

            // PDA account info bindings
            #(#pda_info_bindings)*

            // Mint params
            #mint_bindings

            // Token params
            #token_bindings

            // ATA params
            #ata_bindings

            // Create all accounts via SDK function
            light_account::create_accounts::<
                solana_account_info::AccountInfo<'info>,
                #pda_count,
                #mint_count,
                #token_init_count,
                #ata_init_count,
                _,
            >(
                [#(#pda_init_params),*],
                |__light_config, __current_slot| {
                    #pda_setup_body
                    Ok(())
                },
                #mint_input_expr,
                [#(#token_init_params),*],
                [#(#ata_init_params),*],
                &light_account::SharedAccounts {
                    fee_payer: &__fee_payer_info,
                    cpi_signer: crate::LIGHT_CPI_SIGNER,
                    proof: &#proof_access,
                    program_id: crate::LIGHT_CPI_SIGNER.program_id,
                    compression_config: #compression_config_opt,
                    compressible_config: #compressible_config_opt,
                    rent_sponsor: #rent_sponsor_opt,
                    cpi_authority: #cpi_authority_opt,
                    system_program: #system_program_opt,
                },
                _remaining,
            )?;

            // Reimburse fee payer for rent paid during PDA creation (Anchor-specific)
            #reimburse_block

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
            impl #impl_generics light_account::LightPreInit<light_account::AccountInfo<'info>, #params_type> for #struct_name #ty_generics #where_clause {
                fn light_pre_init(
                    &mut self,
                    _remaining: &[solana_account_info::AccountInfo<'info>],
                    #params_ident: &#params_type,
                ) -> std::result::Result<bool, light_account::LightSdkTypesError> {
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
                ) -> std::result::Result<(), light_account::LightSdkTypesError> {
                    use anchor_lang::ToAccountInfo;
                    #body
                }
            }
        })
    }
}

// ============================================================================
// Helper functions for create_accounts() code generation
// ============================================================================

/// Generate the pda_setup closure body for `create_accounts()`.
///
/// For each PDA field, generates `set_decompressed` calls using the closure's
/// `__light_config` and `__current_slot` parameters.
fn generate_pda_setup_closure_body(fields: &[ParsedPdaField]) -> TokenStream {
    if fields.is_empty() {
        return quote! {};
    }

    let blocks: Vec<TokenStream> = fields.iter().map(|field| {
        let ident = &field.field_name;

        if field.is_zero_copy {
            // AccountLoader: load_init() for zero-copy (Pod) accounts
            quote! {
                {
                    let mut __guard = self.#ident.load_init()
                        .map_err(|_| light_account::LightSdkTypesError::InvalidInstructionData)?;
                    __guard.compression_info =
                        light_account::CompressionInfo::new_from_config(
                            __light_config, __current_slot,
                        );
                }
            }
        } else if field.is_boxed {
            // Box<Account<T>>: deref twice to get inner data, then serialize
            quote! {
                {
                    use light_account::LightAccount;
                    use anchor_lang::AnchorSerialize;
                    // Get account info BEFORE mutable borrow
                    let account_info = self.#ident.to_account_info();
                    {
                        (**self.#ident).set_decompressed(__light_config, __current_slot);
                    }
                    // Serialize to on-chain buffer so data is visible before Anchor exit
                    let mut data = account_info
                        .try_borrow_mut_data()
                        .map_err(|_| light_account::LightSdkTypesError::ConstraintViolation)?;
                    self.#ident.serialize(&mut &mut data[8..])
                        .map_err(|_| light_account::LightSdkTypesError::ConstraintViolation)?;
                }
            }
        } else {
            // Account<T>: deref once to get inner data, then serialize
            quote! {
                {
                    use light_account::LightAccount;
                    use anchor_lang::AnchorSerialize;
                    // Get account info BEFORE mutable borrow
                    let account_info = self.#ident.to_account_info();
                    {
                        (*self.#ident).set_decompressed(__light_config, __current_slot);
                    }
                    // Serialize to on-chain buffer so data is visible before Anchor exit
                    let mut data = account_info
                        .try_borrow_mut_data()
                        .map_err(|_| light_account::LightSdkTypesError::ConstraintViolation)?;
                    self.#ident.serialize(&mut &mut data[8..])
                        .map_err(|_| light_account::LightSdkTypesError::ConstraintViolation)?;
                }
            }
        }
    }).collect();

    quote! { #(#blocks)* }
}

/// Generate mint params bindings and `CreateMintsInput` expression.
///
/// Returns (bindings_code, input_expr) where:
/// - `bindings_code` is the code to build `SingleMintParams` for each mint
/// - `input_expr` is `Some(light_account::CreateMintsInput { ... })`
fn generate_mint_input(mints: &[LightMintField]) -> Result<(TokenStream, TokenStream), syn::Error> {
    let mint_count = mints.len();

    // Per-mint param building code (adapted from LightMintsBuilder::generate_invocation)
    let mint_params_builds: Vec<TokenStream> = mints.iter().enumerate().map(|(idx, mint)| {
        let mint_signer = &mint.mint_signer;
        let authority = &mint.authority;
        let decimals = &mint.decimals;
        let freeze_authority = mint.freeze_authority.as_ref()
            .map(|f| quote! { Some(self.#f.to_account_info().key.to_bytes()) })
            .unwrap_or_else(|| quote! { None });
        let mint_seeds = &mint.mint_seeds;

        let idx_ident = format_ident!("__mint_param_{}", idx);
        let signer_key_ident = format_ident!("__mint_signer_key_{}", idx);
        let mint_seeds_ident = format_ident!("__mint_seeds_{}", idx);
        let mint_seeds_with_bump_ident = format_ident!("__mint_seeds_with_bump_{}", idx);
        let mint_signer_bump_ident = format_ident!("__mint_signer_bump_{}", idx);
        let mint_bump_slice_ident = format_ident!("__mint_bump_slice_{}", idx);
        let auth_bump_slice_ident = format_ident!("__auth_bump_slice_{}", idx);
        let authority_seeds_ident = format_ident!("__authority_seeds_{}", idx);
        let authority_seeds_with_bump_ident = format_ident!("__authority_seeds_with_bump_{}", idx);
        let authority_bump_ident = format_ident!("__authority_bump_{}", idx);
        let token_metadata_ident = format_ident!("__mint_token_metadata_{}", idx);

        // Mint bump derivation
        let mint_bump_derivation = mint.mint_bump.as_ref()
            .map(|b| quote! { let #mint_signer_bump_ident: u8 = #b; })
            .unwrap_or_else(|| {
                quote! {
                    let #mint_signer_bump_ident: u8 = {
                        let (_, bump) = solana_pubkey::Pubkey::find_program_address(
                            #mint_seeds_ident,
                            &solana_pubkey::Pubkey::from(crate::LIGHT_CPI_SIGNER.program_id),
                        );
                        bump
                    };
                }
            });

        // Authority seeds binding
        let authority_seeds_binding = match &mint.authority_seeds {
            Some(seeds) => {
                let authority_bump_derivation = mint.authority_bump.as_ref()
                    .map(|b| quote! { let #authority_bump_ident: u8 = #b; })
                    .unwrap_or_else(|| {
                        quote! {
                            let #authority_bump_ident: u8 = {
                                let base_seeds: &[&[u8]] = #seeds;
                                let (_, bump) = solana_pubkey::Pubkey::find_program_address(
                                    base_seeds,
                                    &solana_pubkey::Pubkey::from(crate::LIGHT_CPI_SIGNER.program_id),
                                );
                                bump
                            };
                        }
                    });
                quote! {
                    let #authority_seeds_ident: &[&[u8]] = #seeds;
                    #authority_bump_derivation
                    let mut #authority_seeds_with_bump_ident: Vec<&[u8]> = #authority_seeds_ident.to_vec();
                    let #auth_bump_slice_ident: &[u8] = &[#authority_bump_ident];
                    #authority_seeds_with_bump_ident.push(#auth_bump_slice_ident);
                    let #authority_seeds_with_bump_ident: Option<Vec<&[u8]>> = Some(#authority_seeds_with_bump_ident);
                }
            }
            None => quote! {
                let #authority_seeds_with_bump_ident: Option<Vec<&[u8]>> = None;
            },
        };

        // Token metadata binding
        let has_metadata = mint.name.is_some();
        let token_metadata_binding = if has_metadata {
            let name_expr = mint.name.as_ref().map(|e| quote! { #e }).expect("mint::name is required when mint::symbol or mint::uri is set");
            let symbol_expr = mint.symbol.as_ref().map(|e| quote! { #e }).expect("mint::symbol is required when mint::name or mint::uri is set");
            let uri_expr = mint.uri.as_ref().map(|e| quote! { #e }).expect("mint::uri is required when mint::name or mint::symbol is set");
            let update_authority_expr = mint.update_authority.as_ref()
                .map(|f| quote! { Some(self.#f.to_account_info().key.to_bytes().into()) })
                .unwrap_or_else(|| quote! { None });
            let additional_metadata_expr = mint.additional_metadata.as_ref()
                .map(|e| quote! { #e })
                .unwrap_or_else(|| quote! { None });

            quote! {
                let #token_metadata_ident: Option<light_account::TokenMetadataInstructionData> = Some(
                    light_account::TokenMetadataInstructionData {
                        update_authority: #update_authority_expr,
                        name: #name_expr,
                        symbol: #symbol_expr,
                        uri: #uri_expr,
                        additional_metadata: #additional_metadata_expr,
                    }
                );
            }
        } else {
            quote! {
                let #token_metadata_ident: Option<light_account::TokenMetadataInstructionData> = None;
            }
        };

        quote! {
            let #signer_key_ident: [u8; 32] = self.#mint_signer.to_account_info().key.to_bytes();

            let #mint_seeds_ident: &[&[u8]] = #mint_seeds;
            #mint_bump_derivation
            let mut #mint_seeds_with_bump_ident: Vec<&[u8]> = #mint_seeds_ident.to_vec();
            let #mint_bump_slice_ident: &[u8] = &[#mint_signer_bump_ident];
            #mint_seeds_with_bump_ident.push(#mint_bump_slice_ident);

            #authority_seeds_binding
            #token_metadata_binding

            let #idx_ident = light_account::SingleMintParams {
                decimals: #decimals,
                mint_authority: self.#authority.to_account_info().key.to_bytes(),
                mint_bump: None,
                freeze_authority: #freeze_authority,
                mint_seed_pubkey: #signer_key_ident,
                authority_seeds: #authority_seeds_with_bump_ident.as_deref(),
                mint_signer_seeds: Some(&#mint_seeds_with_bump_ident[..]),
                token_metadata: #token_metadata_ident.as_ref(),
            };
        }
    }).collect();

    // Authority signer checks
    let authority_signer_checks: Vec<TokenStream> = mints.iter()
        .filter(|m| m.authority_seeds.is_none())
        .map(|mint| {
            let authority = &mint.authority;
            quote! {
                if !self.#authority.to_account_info().is_signer {
                    return Err(anchor_lang::solana_program::program_error::ProgramError::MissingRequiredSignature.into());
                }
            }
        }).collect();

    // Array element identifiers
    let param_idents: Vec<TokenStream> = (0..mint_count)
        .map(|idx| {
            let ident = format_ident!("__mint_param_{}", idx);
            quote! { #ident }
        })
        .collect();

    let mint_seed_account_exprs: Vec<TokenStream> = mints
        .iter()
        .map(|mint| {
            let mint_signer = &mint.mint_signer;
            quote! { self.#mint_signer.to_account_info() }
        })
        .collect();

    let mint_account_exprs: Vec<TokenStream> = mints
        .iter()
        .map(|mint| {
            let field_ident = &mint.field_ident;
            quote! { self.#field_ident.to_account_info() }
        })
        .collect();

    let bindings = quote! {
        #(#mint_params_builds)*
        #(#authority_signer_checks)*
    };

    let input_expr = quote! {
        Some(light_account::CreateMintsInput {
            params: [#(#param_idents),*],
            mint_seed_accounts: [#(#mint_seed_account_exprs),*],
            mint_accounts: [#(#mint_account_exprs),*],
        })
    };

    Ok((bindings, input_expr))
}

/// Generate token vault init param bindings and `TokenInitParam` array elements.
fn generate_token_init_params(fields: &[&TokenAccountField]) -> (TokenStream, Vec<TokenStream>) {
    if fields.is_empty() {
        return (quote! {}, vec![]);
    }

    let mut all_bindings = Vec::new();
    let mut params = Vec::new();

    for (i, field) in fields.iter().enumerate() {
        let field_ident = &field.field_ident;
        let account_info_ident = format_ident!("__token_account_info_{}", i);
        let mint_info_ident = format_ident!("__token_mint_info_{}", i);

        // Bind account info
        all_bindings.push(quote! {
            let #account_info_ident = self.#field_ident.to_account_info();
        });

        // Bind mint info
        let m = field
            .mint
            .as_ref()
            .expect("parser invariant: token init fields always have mint");
        let mint_binding = quote! { let #mint_info_ident = self.#m.to_account_info(); };
        all_bindings.push(mint_binding);

        // Owner expression
        let o = field
            .owner
            .as_ref()
            .expect("parser invariant: token init fields always have owner");
        let owner_expr = quote! { self.#o.to_account_info().key.to_bytes() };

        // Seed bindings and bump derivation
        let token_seeds = &field.seeds;
        let seed_val_idents: Vec<syn::Ident> = (0..token_seeds.len())
            .map(|j| format_ident!("__tseed_{}_{}", i, j))
            .collect();
        let seed_ref_idents: Vec<syn::Ident> = (0..token_seeds.len())
            .map(|j| format_ident!("__tseed_ref_{}_{}", i, j))
            .collect();

        for (j, seed) in token_seeds.iter().enumerate() {
            let val_name = &seed_val_idents[j];
            let ref_name = &seed_ref_idents[j];
            all_bindings.push(quote! {
                let #val_name = #seed;
                let #ref_name: &[u8] = #val_name.as_ref();
            });
        }

        let bump_ident = format_ident!("__token_bump_{}", i);
        let bump_derivation = field
            .bump
            .as_ref()
            .map(|b| quote! { let #bump_ident: u8 = #b; })
            .unwrap_or_else(|| {
                let seed_refs: Vec<&syn::Ident> = seed_ref_idents.iter().collect();
                quote! {
                    let #bump_ident: u8 = {
                        let seeds: &[&[u8]] = &[#(#seed_refs),*];
                        let (_, bump) = solana_pubkey::Pubkey::find_program_address(
                            seeds,
                            &solana_pubkey::Pubkey::from(crate::LIGHT_CPI_SIGNER.program_id),
                        );
                        bump
                    };
                }
            });
        all_bindings.push(bump_derivation);

        let bump_slice_ident = format_ident!("__token_bump_slice_{}", i);
        let seeds_ident = format_ident!("__token_seeds_{}", i);
        all_bindings.push(quote! {
            let #bump_slice_ident: [u8; 1] = [#bump_ident];
        });

        let seed_refs: Vec<&syn::Ident> = seed_ref_idents.iter().collect();
        let seeds_array_expr = quote! { &[#(#seed_refs,)* &#bump_slice_ident[..]] };
        all_bindings.push(quote! {
            let #seeds_ident: &[&[u8]] = #seeds_array_expr;
        });

        params.push(quote! {
            light_account::TokenInitParam {
                account: &#account_info_ident,
                mint: &#mint_info_ident,
                owner: #owner_expr,
                seeds: #seeds_ident,
            }
        });
    }

    let bindings = quote! { #(#all_bindings)* };
    (bindings, params)
}

/// Generate ATA init param bindings and `AtaInitParam` array elements.
fn generate_ata_init_params(fields: &[&AtaField]) -> (TokenStream, Vec<TokenStream>) {
    if fields.is_empty() {
        return (quote! {}, vec![]);
    }

    let mut all_bindings = Vec::new();
    let mut params = Vec::new();

    for (i, field) in fields.iter().enumerate() {
        let field_ident = &field.field_ident;
        let owner = &field.owner;
        let mint = &field.mint;

        let ata_info_ident = format_ident!("__ata_info_{}", i);
        let owner_info_ident = format_ident!("__ata_owner_info_{}", i);
        let mint_info_ident = format_ident!("__ata_mint_info_{}", i);

        all_bindings.push(quote! {
            let #ata_info_ident = self.#field_ident.to_account_info();
            let #owner_info_ident = self.#owner.to_account_info();
            let #mint_info_ident = self.#mint.to_account_info();
        });

        let idempotent_val = field.idempotent;
        params.push(quote! {
            light_account::AtaInitParam {
                ata: &#ata_info_ident,
                owner: &#owner_info_ident,
                mint: &#mint_info_ident,
                idempotent: #idempotent_val,
            }
        });
    }

    let bindings = quote! { #(#all_bindings)* };
    (bindings, params)
}
