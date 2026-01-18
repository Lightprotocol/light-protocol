//! Light mint parsing and code generation.
//!
//! This module handles:
//! - Parsing of #[light_mint(...)] attributes using darling
//! - Code generation for mint_action CPI invocations
//!
//! ## Parsed Attributes
//!
//! Required: `mint_signer`, `authority`, `decimals`, `mint_seeds`
//! Optional: `address_tree_info`, `freeze_authority`, `authority_seeds`, `rent_payment`, `write_top_up`
//!
//! ## Code Generation
//!
//! Two cases for mint_action CPI:
//! - **With CPI context**: Batching mint creation with PDA compression
//! - **Without CPI context**: Mint-only instructions
//!
//! See `CpiContextParts` for what differs between these cases.

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident};

use super::parse::InfraFields;
use crate::rentfree::shared_utils::MetaExpr;

// ============================================================================
// Parsing with darling
// ============================================================================

/// A field marked with #[light_mint(...)]
pub(super) struct LightMintField {
    /// The field name where #[light_mint] is attached (CMint account)
    pub field_ident: Ident,
    /// The mint_signer field (AccountInfo that seeds the mint PDA)
    pub mint_signer: Expr,
    /// The authority for mint operations
    pub authority: Expr,
    /// Decimals for the mint
    pub decimals: Expr,
    /// Address tree info expression
    pub address_tree_info: Expr,
    /// Optional freeze authority
    pub freeze_authority: Option<Ident>,
    /// Signer seeds for the mint_signer PDA (required)
    pub mint_seeds: Expr,
    /// Signer seeds for the authority PDA (optional - if not provided, authority must be a tx signer)
    pub authority_seeds: Option<Expr>,
    /// Rent payment epochs for decompression (default: 2)
    pub rent_payment: Option<Expr>,
    /// Write top-up lamports for decompression (default: 0)
    pub write_top_up: Option<Expr>,
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

/// Arguments inside #[light_mint(...)] parsed by darling.
///
/// Required fields (darling auto-validates): mint_signer, authority, decimals
/// Optional fields: address_tree_info, freeze_authority, mint_seeds, authority_seeds, rent_payment, write_top_up
/// Metadata fields (all optional): name, symbol, uri, update_authority, additional_metadata
#[derive(FromMeta)]
struct LightMintArgs {
    /// The mint_signer field (AccountInfo that seeds the mint PDA) - REQUIRED
    mint_signer: MetaExpr,
    /// The authority for mint operations - REQUIRED
    authority: MetaExpr,
    /// Decimals for the mint - REQUIRED
    decimals: MetaExpr,
    /// Address tree info expression (defaults to params.create_accounts_proof.address_tree_info)
    #[darling(default)]
    address_tree_info: Option<MetaExpr>,
    /// Optional freeze authority (field name, e.g., `freeze_authority = freeze_auth`)
    #[darling(default)]
    freeze_authority: Option<Ident>,
    /// Signer seeds for the mint_signer PDA (required)
    mint_seeds: MetaExpr,
    /// Signer seeds for the authority PDA (optional - if not provided, authority must be a tx signer)
    #[darling(default)]
    authority_seeds: Option<MetaExpr>,
    /// Rent payment epochs for decompression
    #[darling(default)]
    rent_payment: Option<MetaExpr>,
    /// Write top-up lamports for decompression
    #[darling(default)]
    write_top_up: Option<MetaExpr>,
    // Metadata extension fields
    /// Token name for TokenMetadata extension (expression yielding Vec<u8>)
    #[darling(default)]
    name: Option<MetaExpr>,
    /// Token symbol for TokenMetadata extension (expression yielding Vec<u8>)
    #[darling(default)]
    symbol: Option<MetaExpr>,
    /// Token URI for TokenMetadata extension (expression yielding Vec<u8>)
    #[darling(default)]
    uri: Option<MetaExpr>,
    /// Update authority field reference for TokenMetadata extension
    #[darling(default)]
    update_authority: Option<Ident>,
    /// Additional metadata for TokenMetadata extension (expression yielding Option<Vec<AdditionalMetadata>>)
    #[darling(default)]
    additional_metadata: Option<MetaExpr>,
}

/// Validates TokenMetadata field requirements.
///
/// Rules:
/// 1. `name`, `symbol`, `uri` must all be defined together or none
/// 2. `update_authority` and `additional_metadata` require `name`, `symbol`, `uri`
fn validate_metadata_fields(args: &LightMintArgs) -> Result<(), &'static str> {
    let has_name = args.name.is_some();
    let has_symbol = args.symbol.is_some();
    let has_uri = args.uri.is_some();
    let has_update_authority = args.update_authority.is_some();
    let has_additional_metadata = args.additional_metadata.is_some();

    let core_metadata_count = [has_name, has_symbol, has_uri]
        .iter()
        .filter(|&&x| x)
        .count();

    // Rule 1: name, symbol, uri must all be defined together or none
    if core_metadata_count > 0 && core_metadata_count < 3 {
        return Err(
            "TokenMetadata requires all of `name`, `symbol`, and `uri` to be specified together",
        );
    }

    // Rule 2: update_authority and additional_metadata require name, symbol, uri
    if (has_update_authority || has_additional_metadata) && core_metadata_count == 0 {
        return Err(
            "`update_authority` and `additional_metadata` require `name`, `symbol`, and `uri` to also be specified",
        );
    }

    Ok(())
}

/// Parse #[light_mint(...)] attribute from a field.
/// Returns None if no light_mint attribute, Some(LightMintField) if found.
pub(super) fn parse_light_mint_attr(
    field: &syn::Field,
    field_ident: &Ident,
) -> Result<Option<LightMintField>, syn::Error> {
    for attr in &field.attrs {
        if attr.path().is_ident("light_mint") {
            // Use darling to parse the attribute arguments
            let args = LightMintArgs::from_meta(&attr.meta)
                .map_err(|e| syn::Error::new_spanned(attr, e.to_string()))?;

            // Validate metadata fields
            validate_metadata_fields(&args)
                .map_err(|msg| syn::Error::new_spanned(attr, msg))?;

            // address_tree_info defaults to params.create_accounts_proof.address_tree_info
            let address_tree_info = args.address_tree_info.map(Into::into).unwrap_or_else(|| {
                syn::parse_quote!(params.create_accounts_proof.address_tree_info)
            });

            return Ok(Some(LightMintField {
                field_ident: field_ident.clone(),
                mint_signer: args.mint_signer.into(),
                authority: args.authority.into(),
                decimals: args.decimals.into(),
                address_tree_info,
                freeze_authority: args.freeze_authority,
                mint_seeds: args.mint_seeds.into(),
                authority_seeds: args.authority_seeds.map(Into::into),
                rent_payment: args.rent_payment.map(Into::into),
                write_top_up: args.write_top_up.map(Into::into),
                // Metadata extension fields
                name: args.name.map(Into::into),
                symbol: args.symbol.map(Into::into),
                uri: args.uri.map(Into::into),
                update_authority: args.update_authority,
                additional_metadata: args.additional_metadata.map(Into::into),
            }));
        }
    }
    Ok(None)
}

// ============================================================================
// Code Generation
// ============================================================================

/// Quote an optional expression, using default if None.
fn quote_option_or(opt: &Option<Expr>, default: TokenStream) -> TokenStream {
    opt.as_ref().map(|e| quote! { #e }).unwrap_or(default)
}

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
    pub ctoken_config: TokenStream,
    pub ctoken_rent_sponsor: TokenStream,
    pub ctoken_cpi_authority: TokenStream,
}

impl InfraRefs {
    /// Construct from parsed InfraFields, applying defaults for missing fields.
    pub fn from_parsed(infra: &InfraFields) -> Self {
        Self {
            fee_payer: resolve_field_name(&infra.fee_payer, "fee_payer"),
            compression_config: resolve_field_name(&infra.compression_config, "compression_config"),
            ctoken_config: resolve_field_name(&infra.ctoken_config, "ctoken_compressible_config"),
            ctoken_rent_sponsor: resolve_field_name(
                &infra.ctoken_rent_sponsor,
                "ctoken_rent_sponsor",
            ),
            ctoken_cpi_authority: resolve_field_name(
                &infra.ctoken_cpi_authority,
                "ctoken_cpi_authority",
            ),
        }
    }
}

/// Builder for generating code that creates multiple compressed mints using CreateMintsCpi.
///
/// This replaces the previous single-mint LightMintBuilder with support for N mints.
/// Generated code uses `CreateMintsCpi` from light_token_sdk for optimal batching.
///
/// Usage:
/// ```ignore
/// LightMintsBuilder::new(mints, params_ident, &infra)
///     .with_pda_context(pda_count, quote! { #first_pda_output_tree })
///     .generate_invocation()
/// ```
pub(super) struct LightMintsBuilder<'a> {
    mints: &'a [LightMintField],
    params_ident: &'a Ident,
    infra: &'a InfraRefs,
    /// PDA context: (pda_count, output_tree_expr) for batching with PDAs
    pda_context: Option<(usize, TokenStream)>,
}

impl<'a> LightMintsBuilder<'a> {
    /// Create builder with required fields.
    pub fn new(mints: &'a [LightMintField], params_ident: &'a Ident, infra: &'a InfraRefs) -> Self {
        Self {
            mints,
            params_ident,
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
    let params_ident = builder.params_ident;
    let infra = builder.infra;
    let mint_count = mints.len();

    // Infrastructure field references
    let fee_payer = &infra.fee_payer;
    let ctoken_config = &infra.ctoken_config;
    let ctoken_rent_sponsor = &infra.ctoken_rent_sponsor;
    let ctoken_cpi_authority = &infra.ctoken_cpi_authority;

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
                .map(|f| quote! { Some(*self.#f.to_account_info().key) })
                .unwrap_or_else(|| quote! { None });
            let mint_seeds = &mint.mint_seeds;
            let authority_seeds = &mint.authority_seeds;

            let idx_ident = format_ident!("__mint_param_{}", idx);
            let pda_ident = format_ident!("__mint_pda_{}", idx);
            let bump_ident = format_ident!("__mint_bump_{}", idx);
            let signer_key_ident = format_ident!("__mint_signer_key_{}", idx);
            let mint_seeds_ident = format_ident!("__mint_seeds_{}", idx);
            let authority_seeds_ident = format_ident!("__authority_seeds_{}", idx);
            let token_metadata_ident = format_ident!("__mint_token_metadata_{}", idx);

            // Generate optional authority seeds binding
            let authority_seeds_binding = match authority_seeds {
                Some(seeds) => quote! {
                    let #authority_seeds_ident: &[&[u8]] = #seeds;
                    let #authority_seeds_ident = Some(#authority_seeds_ident);
                },
                None => quote! {
                    let #authority_seeds_ident: Option<&[&[u8]]> = None;
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
                    let #token_metadata_ident: Option<light_token_sdk::TokenMetadataInstructionData> = Some(
                        light_token_sdk::TokenMetadataInstructionData {
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
                    let #token_metadata_ident: Option<light_token_sdk::TokenMetadataInstructionData> = None;
                }
            };

            quote! {
                // Mint #idx: derive PDA and build params
                let #signer_key_ident = *self.#mint_signer.to_account_info().key;
                let (#pda_ident, #bump_ident) = light_token_sdk::token::find_mint_address(&#signer_key_ident);

                let #mint_seeds_ident: &[&[u8]] = #mint_seeds;
                #authority_seeds_binding
                #token_metadata_binding

                let __tree_info = &#address_tree_info;

                let #idx_ident = light_token_sdk::token::SingleMintParams {
                    decimals: #decimals,
                    address_merkle_tree_root_index: __tree_info.root_index,
                    mint_authority: *self.#authority.to_account_info().key,
                    compression_address: #pda_ident.to_bytes(),
                    mint: #pda_ident,
                    bump: #bump_ident,
                    freeze_authority: #freeze_authority,
                    mint_seed_pubkey: #signer_key_ident,
                    authority_seeds: #authority_seeds_ident,
                    mint_signer_seeds: Some(#mint_seeds_ident),
                    token_metadata: #token_metadata_ident,
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

    // Get rent_payment and write_top_up from first mint (all mints share same params for now)
    let rent_payment = quote_option_or(&mints[0].rent_payment, quote! { 16u8 });
    let write_top_up = quote_option_or(&mints[0].write_top_up, quote! { 766u32 });

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

            // Extract proof from instruction params
            let __proof: light_token_sdk::CompressedProof = #params_ident.create_accounts_proof.proof.0.clone()
                .expect("proof is required for mint creation");

            // Build SingleMintParams for each mint
            #(#mint_params_builds)*

            // Array of mint params
            let __mint_params: [light_token_sdk::token::SingleMintParams<'_>; #mint_count] = [
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

            // Get tree accounts and indices
            // Output queue for state (compressed accounts) is at tree index 0
            // State merkle tree index comes from the proof (set by pack_proof_for_mints)
            // Address merkle tree index comes from the proof's address_tree_info
            let __tree_info = &#params_ident.create_accounts_proof.address_tree_info;
            let __output_queue_index: u8 = 0;
            let __state_tree_index: u8 = #params_ident.create_accounts_proof.state_tree_index
                .ok_or(anchor_lang::prelude::ProgramError::InvalidArgument)?;
            let __address_tree_index: u8 = __tree_info.address_merkle_tree_pubkey_index;
            let __output_queue = cpi_accounts.get_tree_account_info(__output_queue_index as usize)?;
            let __state_merkle_tree = cpi_accounts.get_tree_account_info(__state_tree_index as usize)?;
            let __address_tree = cpi_accounts.get_tree_account_info(__address_tree_index as usize)?;

            // Build CreateMintsParams with tree indices
            let __create_mints_params = light_token_sdk::token::CreateMintsParams::new(
                &__mint_params,
                __proof,
            )
            .with_rent_payment(#rent_payment)
            .with_write_top_up(#write_top_up) // TODO: discuss to allow a different one per mint.
            .with_cpi_context_offset(#cpi_context_offset)
            .with_output_queue_index(__output_queue_index)
            .with_address_tree_index(__address_tree_index)
            .with_state_tree_index(__state_tree_index);

            // Check authority signers for mints without authority_seeds
            #(#authority_signer_checks)*

            // Build and invoke CreateMintsCpi
            // Seeds are extracted from SingleMintParams internally
            light_token_sdk::token::CreateMintsCpi {
                mint_seed_accounts: &__mint_seed_accounts,
                payer: self.#fee_payer.to_account_info(),
                address_tree: __address_tree.clone(),
                output_queue: __output_queue.clone(),
                state_merkle_tree: __state_merkle_tree.clone(),
                compressible_config: self.#ctoken_config.to_account_info(),
                mints: &__mint_accounts,
                rent_sponsor: self.#ctoken_rent_sponsor.to_account_info(),
                system_accounts: light_token_sdk::token::SystemAccountInfos {
                    light_system_program: cpi_accounts.light_system_program()?.clone(),
                    cpi_authority_pda: self.#ctoken_cpi_authority.to_account_info(),
                    registered_program_pda: cpi_accounts.registered_program_pda()?.clone(),
                    account_compression_authority: cpi_accounts.account_compression_authority()?.clone(),
                    account_compression_program: cpi_accounts.account_compression_program()?.clone(),
                    system_program: cpi_accounts.system_program()?.clone(),
                },
                cpi_context_account: cpi_accounts.cpi_context()?.clone(),
                params: __create_mints_params,
            }
            .invoke()?;
        }
    }
}
