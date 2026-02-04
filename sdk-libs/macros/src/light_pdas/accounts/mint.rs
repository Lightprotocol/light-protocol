//! Light mint code generation.
//!
//! This module handles code generation for mint_action CPI invocations.
//! Parsing is handled by `light_account.rs`.
//!
//! ## Code Generation
//!
//! Two cases for mint_action CPI:
//! - **With CPI context**: Batching mint creation with PDA compression
//! - **Without CPI context**: Mint-only instructions
//!
//! See `CpiContextParts` for what differs between these cases.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident};

use super::parse::InfraFields;

// ============================================================================
// Field Types
// ============================================================================

/// A field marked with #[light_account(init, mint, ...)]
#[derive(Debug)]
pub(crate) struct LightMintField {
    /// The field name where #[light_account(init, mint, ...)] is attached (Mint account)
    pub field_ident: Ident,
    /// The mint_signer field (AccountInfo that seeds the mint PDA)
    pub mint_signer: Expr,
    /// The authority for mint operations
    pub authority: Expr,
    /// Decimals for the mint
    pub decimals: Expr,
    /// Address tree info expression (auto-fetched from CreateAccountsProof)
    pub address_tree_info: Expr,
    /// Optional freeze authority
    pub freeze_authority: Option<Ident>,
    /// Signer seeds for the mint_signer PDA (required, WITHOUT bump - bump is auto-derived or provided via mint_bump)
    pub mint_seeds: Expr,
    /// Optional bump for mint_seeds. If None, auto-derived using find_program_address.
    pub mint_bump: Option<Expr>,
    /// Signer seeds for the authority PDA (optional - if not provided, authority must be a tx signer, WITHOUT bump)
    pub authority_seeds: Option<Expr>,
    /// Optional bump for authority_seeds. If None, auto-derived using find_program_address.
    pub authority_bump: Option<Expr>,
    // Metadata extension fields
    /// Token name for TokenMetadata extension
    pub name: Option<Expr>,
    /// Token symbol for TokenMetadata extension
    pub symbol: Option<Expr>,
    /// Token URI for TokenMetadata extension
    pub uri: Option<Expr>,
    /// Update authority field reference for TokenMetadata extension
    pub update_authority: Option<Ident>,
    /// Additional metadata key-value pairs for TokenMetadata extension
    pub additional_metadata: Option<Expr>,
}

// ============================================================================
// Code Generation
// ============================================================================

/// Resolve optional field name to TokenStream, using default if None.
fn resolve_field_name(field: &Option<syn::Ident>, default: &str) -> TokenStream {
    field.as_ref().map(|f| quote! { #f }).unwrap_or_else(|| {
        let ident = format_ident!("{}", default);
        quote! { #ident }
    })
}

/// Resolved infrastructure field names as TokenStreams.
///
/// Single source of truth for infrastructure fields used across code generation.
pub(super) struct InfraRefs {
    pub fee_payer: TokenStream,
    pub compression_config: TokenStream,
    pub pda_rent_sponsor: TokenStream,
    pub light_token_config: TokenStream,
    pub light_token_rent_sponsor: TokenStream,
    pub light_token_cpi_authority: TokenStream,
}

impl InfraRefs {
    /// Construct from parsed InfraFields, applying defaults for missing fields.
    pub fn from_parsed(infra: &InfraFields) -> Self {
        Self {
            fee_payer: resolve_field_name(&infra.fee_payer, "fee_payer"),
            compression_config: resolve_field_name(&infra.compression_config, "compression_config"),
            pda_rent_sponsor: resolve_field_name(&infra.pda_rent_sponsor, "pda_rent_sponsor"),
            light_token_config: resolve_field_name(&infra.light_token_config, "light_token_config"),
            light_token_rent_sponsor: resolve_field_name(
                &infra.light_token_rent_sponsor,
                "light_token_rent_sponsor",
            ),
            light_token_cpi_authority: resolve_field_name(
                &infra.light_token_cpi_authority,
                "light_token_cpi_authority",
            ),
        }
    }
}

/// Builder for generating code that creates multiple compressed mints using CreateMintsCpi.
///
/// This replaces the previous single-mint LightMintBuilder with support for N mints.
/// Generated code uses `CreateMintsCpi` from light_token for optimal batching.
///
/// Usage:
/// ```ignore
/// LightMintsBuilder::new(mints, &proof_access, &infra)
///     .with_pda_context(pda_count, quote! { #first_pda_output_tree })
///     .generate_invocation()
/// ```
pub(super) struct LightMintsBuilder<'a> {
    mints: &'a [LightMintField],
    /// TokenStream for accessing CreateAccountsProof (e.g., `proof` or `params.create_accounts_proof`)
    proof_access: &'a TokenStream,
    infra: &'a InfraRefs,
    /// PDA context: (pda_count, output_tree_expr) for batching with PDAs
    pda_context: Option<(usize, TokenStream)>,
}

impl<'a> LightMintsBuilder<'a> {
    /// Create builder with required fields.
    pub fn new(
        mints: &'a [LightMintField],
        proof_access: &'a TokenStream,
        infra: &'a InfraRefs,
    ) -> Self {
        Self {
            mints,
            proof_access,
            infra,
            pda_context: None,
        }
    }

    /// Configure for batching with PDAs.
    ///
    /// When PDAs are written to CPI context first, this sets the offset for mint indices
    /// so they don't collide with PDA indices.
    pub fn with_pda_context(mut self, pda_count: usize, output_tree_expr: TokenStream) -> Self {
        self.pda_context = Some((pda_count, output_tree_expr));
        self
    }

    /// Generate CreateMintsCpi invocation code for all mints.
    pub fn generate_invocation(self) -> TokenStream {
        generate_mints_invocation(&self)
    }
}

/// Generate CreateMintsCpi invocation code for multiple mints.
///
/// Flow:
/// 1. For each mint: derive PDA, build SingleMintParams
/// 2. Build arrays for mint_seed_accounts, mints
/// 3. Construct CreateMintsCpi struct
/// 4. Call invoke() - seeds are extracted from SingleMintParams internally
fn generate_mints_invocation(builder: &LightMintsBuilder) -> TokenStream {
    let mints = builder.mints;
    let proof_access = builder.proof_access;
    let infra = builder.infra;
    let mint_count = mints.len();

    // Infrastructure field references
    let fee_payer = &infra.fee_payer;
    let light_token_config = &infra.light_token_config;
    let light_token_rent_sponsor = &infra.light_token_rent_sponsor;
    let light_token_cpi_authority = &infra.light_token_cpi_authority;

    // Determine CPI context offset based on PDA context
    let (cpi_context_offset, output_tree_setup) = match &builder.pda_context {
        Some((pda_count, tree_expr)) => {
            let offset = *pda_count as u8;
            (
                quote! { #offset },
                quote! { let __output_tree_index = #tree_expr; },
            )
        }
        None => (quote! { 0u8 }, quote! {}),
    };

    // Generate code for each mint to build SingleMintParams
    let mint_params_builds: Vec<TokenStream> = mints
        .iter()
        .enumerate()
        .map(|(idx, mint)| {
            let mint_signer = &mint.mint_signer;
            let authority = &mint.authority;
            let decimals = &mint.decimals;
            let address_tree_info = &mint.address_tree_info;
            let freeze_authority = mint
                .freeze_authority
                .as_ref()
                .map(|f| quote! { Some(self.#f.to_account_info().key.to_bytes()) })
                .unwrap_or_else(|| quote! { None });
            let mint_seeds = &mint.mint_seeds;
            let authority_seeds = &mint.authority_seeds;

            let idx_ident = format_ident!("__mint_param_{}", idx);
            let signer_key_ident = format_ident!("__mint_signer_key_{}", idx);
            let mint_seeds_ident = format_ident!("__mint_seeds_{}", idx);
            let mint_seeds_with_bump_ident = format_ident!("__mint_seeds_with_bump_{}", idx);
            let mint_signer_bump_ident = format_ident!("__mint_signer_bump_{}", idx);
            let authority_seeds_ident = format_ident!("__authority_seeds_{}", idx);
            let authority_seeds_with_bump_ident = format_ident!("__authority_seeds_with_bump_{}", idx);
            let authority_bump_ident = format_ident!("__authority_bump_{}", idx);
            let token_metadata_ident = format_ident!("__mint_token_metadata_{}", idx);

            // Generate mint_seeds binding with bump derivation/appending
            // User provides base seeds WITHOUT bump, we auto-derive or use provided bump
            let mint_bump_derivation = mint.mint_bump
                .as_ref()
                .map(|b| quote! { let #mint_signer_bump_ident: u8 = #b; })
                .unwrap_or_else(|| {
                    // Auto-derive bump from mint_seeds
                    quote! {
                        let #mint_signer_bump_ident: u8 = {
                            let (_, bump) = solana_pubkey::Pubkey::find_program_address(#mint_seeds_ident, &solana_pubkey::Pubkey::from(crate::LIGHT_CPI_SIGNER.program_id));
                            bump
                        };
                    }
                });

            // Generate optional authority seeds binding with bump derivation/appending
            let authority_seeds_binding = match authority_seeds {
                Some(seeds) => {
                    let authority_bump_derivation = mint.authority_bump
                        .as_ref()
                        .map(|b| quote! { let #authority_bump_ident: u8 = #b; })
                        .unwrap_or_else(|| {
                            // Auto-derive bump from authority_seeds
                            quote! {
                                let #authority_bump_ident: u8 = {
                                    let base_seeds: &[&[u8]] = #seeds;
                                    let (_, bump) = solana_pubkey::Pubkey::find_program_address(base_seeds, &solana_pubkey::Pubkey::from(crate::LIGHT_CPI_SIGNER.program_id));
                                    bump
                                };
                            }
                        });
                    quote! {
                        let #authority_seeds_ident: &[&[u8]] = #seeds;
                        #authority_bump_derivation
                        // Build Vec with bump appended (using Vec since we can't create fixed-size array at compile time)
                        let mut #authority_seeds_with_bump_ident: Vec<&[u8]> = #authority_seeds_ident.to_vec();
                        let __auth_bump_slice: &[u8] = &[#authority_bump_ident];
                        #authority_seeds_with_bump_ident.push(__auth_bump_slice);
                        let #authority_seeds_with_bump_ident: Option<Vec<&[u8]>> = Some(#authority_seeds_with_bump_ident);
                    }
                },
                None => quote! {
                    let #authority_seeds_with_bump_ident: Option<Vec<&[u8]>> = None;
                },
            };

            // Check if metadata is present (validation guarantees name/symbol/uri are all-or-nothing)
            let has_metadata = mint.name.is_some();

            // Generate token_metadata binding
            let token_metadata_binding = if has_metadata {
                // name, symbol, uri are guaranteed to be present by validation
                let name_expr = mint.name.as_ref().map(|e| quote! { #e }).unwrap();
                let symbol_expr = mint.symbol.as_ref().map(|e| quote! { #e }).unwrap();
                let uri_expr = mint.uri.as_ref().map(|e| quote! { #e }).unwrap();
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
                // Mint #idx: build params (mint and compression_address derived internally)
                let #signer_key_ident: [u8; 32] = self.#mint_signer.to_account_info().key.to_bytes();

                // Bind base mint_seeds (WITHOUT bump) and derive/get bump
                let #mint_seeds_ident: &[&[u8]] = #mint_seeds;
                #mint_bump_derivation
                // Build Vec with bump appended
                let mut #mint_seeds_with_bump_ident: Vec<&[u8]> = #mint_seeds_ident.to_vec();
                let __mint_bump_slice: &[u8] = &[#mint_signer_bump_ident];
                #mint_seeds_with_bump_ident.push(__mint_bump_slice);

                #authority_seeds_binding
                #token_metadata_binding

                let __tree_info = &#address_tree_info;

                let #idx_ident = light_account::SingleMintParams {
                    decimals: #decimals,
                    mint_authority: self.#authority.to_account_info().key.to_bytes(),
                    mint_bump: None, // derived internally from mint_seed_pubkey
                    freeze_authority: #freeze_authority,
                    mint_seed_pubkey: #signer_key_ident,
                    authority_seeds: #authority_seeds_with_bump_ident.as_deref(),
                    mint_signer_seeds: Some(&#mint_seeds_with_bump_ident[..]),
                    token_metadata: #token_metadata_ident.as_ref(),
                };
            }
        })
        .collect();

    // Generate array of SingleMintParams
    let param_idents: Vec<TokenStream> = (0..mint_count)
        .map(|idx| {
            let ident = format_ident!("__mint_param_{}", idx);
            quote! { #ident }
        })
        .collect();

    // Generate array of mint seed AccountInfos
    let mint_seed_account_exprs: Vec<TokenStream> = mints
        .iter()
        .map(|mint| {
            let mint_signer = &mint.mint_signer;
            quote! { self.#mint_signer.to_account_info() }
        })
        .collect();

    // Generate array of mint AccountInfos
    let mint_account_exprs: Vec<TokenStream> = mints
        .iter()
        .map(|mint| {
            let field_ident = &mint.field_ident;
            quote! { self.#field_ident.to_account_info() }
        })
        .collect();

    // Authority signer check for mints without authority_seeds
    let authority_signer_checks: Vec<TokenStream> = mints
        .iter()
        .filter(|m| m.authority_seeds.is_none())
        .map(|mint| {
            let authority = &mint.authority;
            quote! {
                if !self.#authority.to_account_info().is_signer {
                    return Err(anchor_lang::solana_program::program_error::ProgramError::MissingRequiredSignature.into());
                }
            }
        })
        .collect();

    quote! {
        {
            #output_tree_setup

            // Build SingleMintParams for each mint
            #(#mint_params_builds)*

            // Array of mint params
            let __mint_params: [light_account::SingleMintParams<'_>; #mint_count] = [
                #(#param_idents),*
            ];

            // Array of mint seed AccountInfos
            let __mint_seed_accounts: [solana_account_info::AccountInfo<'info>; #mint_count] = [
                #(#mint_seed_account_exprs),*
            ];

            // Array of mint AccountInfos
            let __mint_accounts: [solana_account_info::AccountInfo<'info>; #mint_count] = [
                #(#mint_account_exprs),*
            ];

            // Check authority signers for mints without authority_seeds
            #(#authority_signer_checks)*

            // Build CreateMints struct and invoke
            light_account::CreateMints {
                mints: &__mint_params,
                proof_data: &#proof_access,
                mint_seed_accounts: &__mint_seed_accounts,
                mint_accounts: &__mint_accounts,
                static_accounts: light_account::CreateMintsStaticAccounts {
                    fee_payer: &self.#fee_payer.to_account_info(),
                    compressible_config: &self.#light_token_config.to_account_info(),
                    rent_sponsor: &self.#light_token_rent_sponsor.to_account_info(),
                    cpi_authority: &self.#light_token_cpi_authority.to_account_info(),
                },
                cpi_context_offset: #cpi_context_offset,
            }
            .invoke(&cpi_accounts)?;
        }
    }
}
