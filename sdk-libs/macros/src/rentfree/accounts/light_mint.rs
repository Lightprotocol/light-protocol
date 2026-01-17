//! Light mint parsing and code generation.
//!
//! This module handles:
//! - Parsing of #[light_mint(...)] attributes using darling
//! - Code generation for mint_action CPI invocations

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, Ident};

// ============================================================================
// Parsing with darling
// ============================================================================

/// Wrapper for syn::Expr that implements darling's FromMeta trait.
/// Enables darling to parse arbitrary expressions in attributes like
/// `#[light_mint(mint_signer = self.authority)]`.
#[derive(Clone)]
struct MetaExpr(Expr);

impl FromMeta for MetaExpr {
    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        Ok(MetaExpr(expr.clone()))
    }
}

impl From<MetaExpr> for Expr {
    fn from(meta: MetaExpr) -> Expr {
        meta.0
    }
}

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
    pub freeze_authority: Option<Expr>,
    /// Signer seeds for the mint_signer PDA (required if mint_signer is a PDA)
    pub signer_seeds: Option<Expr>,
    /// Rent payment epochs for decompression (default: 2)
    pub rent_payment: Option<Expr>,
    /// Write top-up lamports for decompression (default: 0)
    pub write_top_up: Option<Expr>,
}

/// Arguments inside #[light_mint(...)] parsed by darling.
///
/// Required fields (darling auto-validates): mint_signer, authority, decimals
/// Optional fields: address_tree_info, freeze_authority, signer_seeds, rent_payment, write_top_up
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
    /// Optional freeze authority
    #[darling(default)]
    freeze_authority: Option<MetaExpr>,
    /// Signer seeds for the mint_signer PDA (required if mint_signer is a PDA)
    #[darling(default)]
    signer_seeds: Option<MetaExpr>,
    /// Rent payment epochs for decompression
    #[darling(default)]
    rent_payment: Option<MetaExpr>,
    /// Write top-up lamports for decompression
    #[darling(default)]
    write_top_up: Option<MetaExpr>,
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

            // address_tree_info defaults to params.create_accounts_proof.address_tree_info
            let address_tree_info = args
                .address_tree_info
                .map(Into::into)
                .unwrap_or_else(|| syn::parse_quote!(params.create_accounts_proof.address_tree_info));

            return Ok(Some(LightMintField {
                field_ident: field_ident.clone(),
                mint_signer: args.mint_signer.into(),
                authority: args.authority.into(),
                decimals: args.decimals.into(),
                address_tree_info,
                freeze_authority: args.freeze_authority.map(Into::into),
                signer_seeds: args.signer_seeds.map(Into::into),
                rent_payment: args.rent_payment.map(Into::into),
                write_top_up: args.write_top_up.map(Into::into),
            }));
        }
    }
    Ok(None)
}

// ============================================================================
// Code Generation
// ============================================================================

/// Default rent payment period in epochs (how long to prepay rent for decompressed accounts).
const DEFAULT_RENT_PAYMENT_EPOCHS: u8 = 2;

/// Default write top-up in lamports (additional lamports for write operations during decompression).
const DEFAULT_WRITE_TOP_UP_LAMPORTS: u32 = 0;

/// Generate token stream for signer seeds (explicit or empty default)
fn generate_signer_seeds_tokens(signer_seeds: &Option<Expr>) -> TokenStream {
    if let Some(seeds) = signer_seeds {
        quote! { #seeds }
    } else {
        quote! { &[] as &[&[u8]] }
    }
}

/// Generate token stream for freeze authority expression
fn generate_freeze_authority_tokens(freeze_authority: &Option<Expr>) -> TokenStream {
    if let Some(freeze_auth) = freeze_authority {
        quote! { Some(*self.#freeze_auth.to_account_info().key) }
    } else {
        quote! { None }
    }
}

/// Generate token stream for rent payment with default
fn generate_rent_payment_tokens(rent_payment: &Option<Expr>) -> TokenStream {
    if let Some(rent) = rent_payment {
        quote! { #rent }
    } else {
        let default = DEFAULT_RENT_PAYMENT_EPOCHS;
        quote! { #default }
    }
}

/// Generate token stream for write top-up with default
fn generate_write_top_up_tokens(write_top_up: &Option<Expr>) -> TokenStream {
    if let Some(top_up) = write_top_up {
        quote! { #top_up }
    } else {
        let default = DEFAULT_WRITE_TOP_UP_LAMPORTS;
        quote! { #default }
    }
}

/// Configuration for mint_action CPI generation
pub(super) struct MintActionConfig<'a> {
    pub mint: &'a LightMintField,
    pub params_ident: &'a syn::Ident,
    pub fee_payer: &'a TokenStream,
    pub ctoken_config: &'a TokenStream,
    pub ctoken_rent_sponsor: &'a TokenStream,
    pub light_token_program: &'a TokenStream,
    pub ctoken_cpi_authority: &'a TokenStream,
    /// CPI context config: (output_tree_index_expr, assigned_account_index)
    /// None = no CPI context (mints-only case)
    pub cpi_context: Option<(TokenStream, u8)>,
}

/// Generate mint_action invocation with optional CPI context
pub(super) fn generate_mint_action_invocation(config: &MintActionConfig) -> TokenStream {
    let MintActionConfig {
        mint,
        params_ident,
        fee_payer,
        ctoken_config,
        ctoken_rent_sponsor,
        light_token_program,
        ctoken_cpi_authority,
        cpi_context,
    } = config;

    let mint_field_ident = &mint.field_ident;
    let mint_signer = &mint.mint_signer;
    let authority = &mint.authority;
    let decimals = &mint.decimals;
    let address_tree_info = &mint.address_tree_info;

    let signer_seeds_tokens = generate_signer_seeds_tokens(&mint.signer_seeds);
    let freeze_authority_tokens = generate_freeze_authority_tokens(&mint.freeze_authority);
    let rent_payment_tokens = generate_rent_payment_tokens(&mint.rent_payment);
    let write_top_up_tokens = generate_write_top_up_tokens(&mint.write_top_up);

    // Queue access differs based on CPI context presence
    let queue_access = if cpi_context.is_some() {
        quote! { __output_tree_index as usize }
    } else {
        quote! { __tree_info.address_queue_pubkey_index as usize }
    };

    // CPI context setup block (empty if no CPI context)
    let cpi_context_setup = if let Some((output_tree_expr, _)) = cpi_context {
        quote! {
            let __output_tree_index = #output_tree_expr;
        }
    } else {
        quote! {}
    };

    // CPI context chain method (empty if no CPI context)
    let cpi_context_chain = if let Some((output_tree_expr, assigned_idx)) = cpi_context {
        quote! {
            .with_cpi_context(light_token_interface::instructions::mint_action::CpiContext {
                address_tree_pubkey: __tree_pubkey.to_bytes(),
                set_context: false,
                first_set_context: false,
                in_tree_index: #output_tree_expr + 1,
                in_queue_index: #output_tree_expr,
                out_queue_index: #output_tree_expr,
                token_out_queue_index: 0,
                assigned_account_index: #assigned_idx,
                read_only_address_trees: [0; 4],
            })
        }
    } else {
        quote! {}
    };

    // CPI context on meta_config (only if CPI context present)
    let meta_cpi_context = if cpi_context.is_some() {
        quote! { meta_config.cpi_context = Some(*cpi_accounts.cpi_context()?.key); }
    } else {
        quote! {}
    };

    // Use `let mut` only when CPI context needs modification
    let instruction_data_binding = if cpi_context.is_some() {
        quote! { let mut instruction_data }
    } else {
        quote! { let instruction_data }
    };

    quote! {
        {
            let __tree_info = &#address_tree_info;
            let address_tree = cpi_accounts.get_tree_account_info(__tree_info.address_merkle_tree_pubkey_index as usize)?;
            #cpi_context_setup
            let output_queue = cpi_accounts.get_tree_account_info(#queue_access)?;
            let __tree_pubkey: solana_pubkey::Pubkey = light_sdk::light_account_checks::AccountInfoTrait::pubkey(address_tree);

            let mint_signer_key = self.#mint_signer.to_account_info().key;
            let (mint_pda, _cmint_bump) = light_token_sdk::token::find_mint_address(mint_signer_key);

            let __proof: light_token_sdk::CompressedProof = #params_ident.create_accounts_proof.proof.0.clone()
                .expect("proof is required for mint creation");

            let __freeze_authority: Option<solana_pubkey::Pubkey> = #freeze_authority_tokens;

            let compressed_mint_data = light_token_interface::instructions::mint_action::MintInstructionData {
                supply: 0,
                decimals: #decimals,
                metadata: light_token_interface::state::MintMetadata {
                    version: 3,
                    mint: mint_pda.to_bytes().into(),
                    mint_decompressed: false,
                    mint_signer: mint_signer_key.to_bytes(),
                    bump: _cmint_bump,
                },
                mint_authority: Some((*self.#authority.to_account_info().key).to_bytes().into()),
                freeze_authority: __freeze_authority.map(|a| a.to_bytes().into()),
                extensions: None,
            };

            #instruction_data_binding = light_token_interface::instructions::mint_action::MintActionCompressedInstructionData::new_mint(
                __tree_info.root_index,
                __proof,
                compressed_mint_data,
            )
            .with_decompress_mint(light_token_interface::instructions::mint_action::DecompressMintAction {
                rent_payment: #rent_payment_tokens,
                write_top_up: #write_top_up_tokens,
            })
            #cpi_context_chain;

            let mut meta_config = light_token_sdk::compressed_token::mint_action::MintActionMetaConfig::new_create_mint(
                *self.#fee_payer.to_account_info().key,
                *self.#authority.to_account_info().key,
                *mint_signer_key,
                __tree_pubkey,
                *output_queue.key,
            )
            .with_compressible_mint(
                mint_pda,
                *self.#ctoken_config.to_account_info().key,
                *self.#ctoken_rent_sponsor.to_account_info().key,
            );

            #meta_cpi_context

            let account_metas = meta_config.to_account_metas();

            use light_compressed_account::instruction_data::traits::LightInstructionData;
            let ix_data = instruction_data.data()
                .map_err(|_| light_sdk::error::LightSdkError::Borsh)?;

            let mint_action_ix = anchor_lang::solana_program::instruction::Instruction {
                program_id: solana_pubkey::Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID),
                accounts: account_metas,
                data: ix_data,
            };

            let mut account_infos = cpi_accounts.to_account_infos();
            account_infos.push(self.#light_token_program.to_account_info());
            account_infos.push(self.#ctoken_cpi_authority.to_account_info());
            account_infos.push(self.#mint_field_ident.to_account_info());
            account_infos.push(self.#ctoken_config.to_account_info());
            account_infos.push(self.#ctoken_rent_sponsor.to_account_info());
            account_infos.push(self.#authority.to_account_info());
            account_infos.push(self.#mint_signer.to_account_info());
            account_infos.push(self.#fee_payer.to_account_info());

            let signer_seeds: &[&[u8]] = #signer_seeds_tokens;
            if signer_seeds.is_empty() {
                anchor_lang::solana_program::program::invoke(&mint_action_ix, &account_infos)?;
            } else {
                anchor_lang::solana_program::program::invoke_signed(&mint_action_ix, &account_infos, &[signer_seeds])?;
            }
        }
    }
}
