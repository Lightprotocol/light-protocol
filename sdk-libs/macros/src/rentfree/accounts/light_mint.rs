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
}

/// Arguments inside #[light_mint(...)] parsed by darling.
///
/// Required fields (darling auto-validates): mint_signer, authority, decimals
/// Optional fields: address_tree_info, freeze_authority, mint_seeds, authority_seeds, rent_payment, write_top_up
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
    pub light_token_program: TokenStream,
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
            light_token_program: resolve_field_name(&infra.ctoken_program, "light_token_program"),
            ctoken_cpi_authority: resolve_field_name(
                &infra.ctoken_cpi_authority,
                "ctoken_cpi_authority",
            ),
        }
    }
}

/// Parts of generated code that differ based on CPI context presence.
///
/// - **With CPI context**: Used when batching mint creation with PDA compression.
///   The mint shares output tree with PDAs, uses assigned_account_index for ordering.
///
/// - **Without CPI context**: Used for mint-only instructions.
///   The mint uses its own address tree info directly.
struct CpiContextParts {
    /// Queue access expression (how to get output queue index)
    queue_access: TokenStream,
    /// Setup block (defines __output_tree_index if needed)
    setup: TokenStream,
    /// Method chain for CPI context configuration on instruction data
    chain: TokenStream,
    /// Meta config assignment (sets cpi_context on meta_config)
    meta_assignment: TokenStream,
    /// Variable binding for instruction_data (mut or not)
    data_binding: TokenStream,
}

impl CpiContextParts {
    fn new(cpi_context: &Option<(TokenStream, u8)>) -> Self {
        match cpi_context {
            Some((tree_expr, assigned_idx)) => Self {
                // With CPI context - batching with PDAs
                queue_access: quote! { __output_tree_index as usize },
                setup: quote! { let __output_tree_index = #tree_expr; },
                chain: quote! {
                    .with_cpi_context(light_token_interface::instructions::mint_action::CpiContext {
                        address_tree_pubkey: __tree_pubkey.to_bytes(),
                        set_context: false,
                        first_set_context: false,
                        in_tree_index: #tree_expr + 1,
                        in_queue_index: #tree_expr,
                        out_queue_index: #tree_expr,
                        token_out_queue_index: 0,
                        assigned_account_index: #assigned_idx,
                        read_only_address_trees: [0; 4],
                    })
                },
                meta_assignment: quote! { meta_config.cpi_context = Some(*cpi_accounts.cpi_context()?.key); },
                data_binding: quote! { let mut instruction_data },
            },
            None => Self {
                // Without CPI context - mint only
                queue_access: quote! { __tree_info.address_queue_pubkey_index as usize },
                setup: quote! {},
                chain: quote! {},
                meta_assignment: quote! {},
                data_binding: quote! { let instruction_data },
            },
        }
    }
}

/// Builder for mint code generation.
///
/// Usage:
/// ```ignore
/// LightMintBuilder::new(mint, params_ident, &infra)
///     .with_cpi_context(quote! { #first_pda_output_tree }, mint_assigned_index)
///     .generate_invocation()
/// ```
pub(super) struct LightMintBuilder<'a> {
    mint: &'a LightMintField,
    params_ident: &'a Ident,
    infra: &'a InfraRefs,
    cpi_context: Option<(TokenStream, u8)>,
}

impl<'a> LightMintBuilder<'a> {
    /// Create builder with required fields.
    pub fn new(mint: &'a LightMintField, params_ident: &'a Ident, infra: &'a InfraRefs) -> Self {
        Self {
            mint,
            params_ident,
            infra,
            cpi_context: None,
        }
    }

    /// Configure CPI context for batching with PDAs.
    pub fn with_cpi_context(mut self, tree_expr: TokenStream, assigned_idx: u8) -> Self {
        self.cpi_context = Some((tree_expr, assigned_idx));
        self
    }

    /// Generate mint_action CPI invocation code.
    pub fn generate_invocation(self) -> TokenStream {
        generate_mint_invocation(&self)
    }
}

/// Generate mint_action invocation code.
///
/// This is the main orchestration function. Shows the high-level flow:
/// 1. Determine CPI context parts (single branching point for all CPI differences)
/// 2. Generate optional field expressions (signer_seeds, freeze_authority, etc.)
/// 3. Generate the complete mint_action CPI invocation block
fn generate_mint_invocation(builder: &LightMintBuilder) -> TokenStream {
    let mint = builder.mint;
    let params_ident = builder.params_ident;
    let infra = &builder.infra;

    // 2. Generate optional field expressions
    let mint_seeds = &mint.mint_seeds;
    let authority_seeds = &mint.authority_seeds;
    let freeze_authority = mint
        .freeze_authority
        .as_ref()
        .map(|f| quote! { Some(*self.#f.to_account_info().key) })
        .unwrap_or_else(|| quote! { None });
    let rent_payment = quote_option_or(&mint.rent_payment, quote! { 2u8 });
    let write_top_up = quote_option_or(&mint.write_top_up, quote! { 0u32 });

    // 3. Generate the mint_action CPI block
    let mint_field_ident = &mint.field_ident;
    let mint_signer = &mint.mint_signer;
    let authority = &mint.authority;
    let decimals = &mint.decimals;
    let address_tree_info = &mint.address_tree_info;

    let fee_payer = &infra.fee_payer;
    let ctoken_config = &infra.ctoken_config;
    let ctoken_rent_sponsor = &infra.ctoken_rent_sponsor;
    let light_token_program = &infra.light_token_program;
    let ctoken_cpi_authority = &infra.ctoken_cpi_authority;

    // 1. Determine CPI context parts (single branching point)
    let cpi = CpiContextParts::new(&builder.cpi_context);

    // Destructure CPI parts for use in quote
    let CpiContextParts {
        queue_access,
        setup: cpi_setup,
        chain: cpi_chain,
        meta_assignment: cpi_meta_assignment,
        data_binding,
    } = cpi;

    // Generate invoke_signed call with appropriate signer seeds
    let invoke_signed_call = match authority_seeds {
        Some(auth_seeds) => {
            quote! {
                let authority_seeds: &[&[u8]] = #auth_seeds;
                anchor_lang::solana_program::program::invoke_signed(
                    &mint_action_ix,
                    &account_infos,
                    &[mint_seeds, authority_seeds]
                )?;
            }
        }
        None => {
            // authority_seeds not provided - authority must be a transaction signer
            quote! {
                // Verify authority is a signer since authority_seeds was not provided
                if !self.#authority.to_account_info().is_signer {
                    return Err(anchor_lang::solana_program::program_error::ProgramError::MissingRequiredSignature.into());
                }
                anchor_lang::solana_program::program::invoke_signed(
                    &mint_action_ix,
                    &account_infos,
                    &[mint_seeds]
                )?;
            }
        }
    };

    // -------------------------------------------------------------------------
    // Generated code block for mint_action CPI invocation.
    //
    // Interpolated variables from CpiContextParts (see struct for with/without cases):
    //   #cpi_setup          - defines __output_tree_index when batching with PDAs
    //   #queue_access       - expression to get output queue index
    //   #data_binding       - "let mut" (with CPI) or "let" (without CPI)
    //   #cpi_chain          - adds .with_cpi_context(...) when batching
    //   #cpi_meta_assignment - sets meta_config.cpi_context when batching
    //
    // Interpolated variables from #[light_mint(...)] attributes:
    //   #address_tree_info  - tree info (default: params.create_accounts_proof.address_tree_info)
    //   #mint_signer        - field that seeds the mint PDA
    //   #authority          - mint authority field
    //   #decimals           - mint decimals
    //   #freeze_authority   - optional freeze authority (Some(*self.field.key) or None)
    //   #rent_payment       - rent epochs for decompression (default: 2u8)
    //   #write_top_up       - write top-up lamports (default: 0u32)
    //   #mint_seeds         - PDA signer seeds for mint_signer (default: &[] as &[&[u8]])
    //   #authority_seeds    - PDA signer seeds for authority (optional, if authority is a PDA)
    //
    // Interpolated variables from infrastructure fields:
    //   #fee_payer, #ctoken_config, #ctoken_rent_sponsor,
    //   #light_token_program, #ctoken_cpi_authority, #mint_field_ident
    // -------------------------------------------------------------------------
    quote! {
        {
            // Step 1: Resolve tree accounts
            let __tree_info = &#address_tree_info;
            let address_tree = cpi_accounts.get_tree_account_info(__tree_info.address_merkle_tree_pubkey_index as usize)?;
            #cpi_setup
            let output_queue = cpi_accounts.get_tree_account_info(#queue_access)?;
            let __tree_pubkey: solana_pubkey::Pubkey = light_sdk::light_account_checks::AccountInfoTrait::pubkey(address_tree);

            // Step 2: Derive mint PDA from mint_signer
            let mint_signer_key = self.#mint_signer.to_account_info().key;
            let (mint_pda, _cmint_bump) = light_token_sdk::token::find_mint_address(mint_signer_key);

            // Step 3: Extract proof from instruction params
            let __proof: light_token_sdk::CompressedProof = #params_ident.create_accounts_proof.proof.0.clone()
                .expect("proof is required for mint creation");

            let __freeze_authority: Option<solana_pubkey::Pubkey> = #freeze_authority;

            // Step 4: Build mint instruction data
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

            // Step 5: Build compressed instruction data with decompress config
            #data_binding = light_token_interface::instructions::mint_action::MintActionCompressedInstructionData::new_mint(
                __tree_info.root_index,
                __proof,
                compressed_mint_data,
            )
            .with_decompress_mint(light_token_interface::instructions::mint_action::DecompressMintAction {
                rent_payment: #rent_payment,
                write_top_up: #write_top_up,
            })
            #cpi_chain;

            // Step 6: Build account metas for CPI
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

            #cpi_meta_assignment

            let account_metas = meta_config.to_account_metas();

            // Step 7: Serialize instruction data
            use light_compressed_account::instruction_data::traits::LightInstructionData;
            let ix_data = instruction_data.data()
                .map_err(|_| light_sdk::error::LightSdkError::Borsh)?;

            // Step 8: Build the CPI instruction
            let mint_action_ix = anchor_lang::solana_program::instruction::Instruction {
                program_id: solana_pubkey::Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID),
                accounts: account_metas,
                data: ix_data,
            };

            // Step 9: Collect account infos for CPI
            let mut account_infos = cpi_accounts.to_account_infos();
            account_infos.push(self.#light_token_program.to_account_info());
            account_infos.push(self.#ctoken_cpi_authority.to_account_info());
            account_infos.push(self.#mint_field_ident.to_account_info());
            account_infos.push(self.#ctoken_config.to_account_info());
            account_infos.push(self.#ctoken_rent_sponsor.to_account_info());
            account_infos.push(self.#authority.to_account_info());
            account_infos.push(self.#mint_signer.to_account_info());
            account_infos.push(self.#fee_payer.to_account_info());

            // Step 10: Invoke CPI with signer seeds
            let mint_seeds: &[&[u8]] = #mint_seeds;
            #invoke_signed_call
        }
    }
}
