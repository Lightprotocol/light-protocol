//! Unified #[light_account(...)] attribute parsing.
//!
//! This module provides a single unified syntax for all Light Protocol account types:
//! - `#[light_account(init)]` - PDAs
//! - `#[light_account(init, mint, ...)]` - Light Mints
//! - `#[light_account(token, ...)]` - Light token accounts
//!
//! ## Syntax (Anchor-style namespace::key)
//!
//! All parameters require a namespace prefix matching the account type:
//!
//! ### Token Account
//! ```ignore
//! #[light_account(init, token,
//!     token::authority = [VAULT_SEED, self.offer.key()],
//!     token::mint = token_mint_a,
//!     token::owner = authority,
//!     token::bump = params.vault_bump
//! )]
//! ```
//!
//! ### Associated Token Account
//! ```ignore
//! #[light_account(init, associated_token,
//!     associated_token::authority = owner,
//!     associated_token::mint = mint,
//!     associated_token::bump = params.ata_bump
//! )]
//! ```
//!
//! ### Mint
//! ```ignore
//! #[light_account(init, mint,
//!     mint::signer = mint_signer,
//!     mint::authority = authority,
//!     mint::decimals = params.decimals,
//!     mint::seeds = &[MINT_SIGNER_SEED, self.authority.key().as_ref()],
//!     mint::bump = params.mint_signer_bump
//! )]
//! ```
//!
//! Note: Token fields are NOT processed here - they're handled by seed_extraction.rs
//! in the light_program macro. This parser returns None for token fields.

use syn::{
    parse::{Parse, ParseStream},
    Error, Expr, Field, Ident, Token, Type,
};

use super::mint::LightMintField;
pub(super) use crate::light_pdas::account::seed_extraction::extract_account_inner_type;
use crate::light_pdas::light_account_keywords::{
    is_shorthand_key, is_standalone_keyword, missing_namespace_error, valid_keys_for_namespace,
    validate_namespaced_key,
};

// ============================================================================
// Account Type Classification
// ============================================================================

/// Account type specifier parsed from the attribute.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum LightAccountType {
    #[default]
    Pda, // Default (no type specifier) - for PDAs
    Mint,            // `mint` keyword - for compressed mints
    Token,           // `token` keyword - for token accounts
    AssociatedToken, // `associated_token` keyword - for ATAs
}

impl LightAccountType {
    /// Get the namespace string for this account type.
    pub fn namespace(&self) -> &'static str {
        match self {
            LightAccountType::Pda => "pda",
            LightAccountType::Mint => "mint",
            LightAccountType::Token => "token",
            LightAccountType::AssociatedToken => "associated_token",
        }
    }
}

// ============================================================================
// Unified Parsed Result
// ============================================================================

/// Unified representation of a #[light_account(...)] field.
#[derive(Debug)]
pub enum LightAccountField {
    Pda(Box<PdaField>),
    Mint(Box<LightMintField>),
    TokenAccount(Box<TokenAccountField>),
    AssociatedToken(Box<AtaField>),
}

/// A field marked with #[light_account(init)] (PDA).
#[derive(Debug)]
pub struct PdaField {
    pub ident: Ident,
    /// The inner type T from Account<'info, T> or Box<Account<'info, T>>
    pub inner_type: Type,
    pub address_tree_info: Expr,
    pub output_tree: Expr,
    /// True if the field is Box<Account<T>>, false if Account<T>
    pub is_boxed: bool,
}

/// A field marked with #[light_account([init,] token, ...)] (Token Account).
#[derive(Clone, Debug)]
pub struct TokenAccountField {
    pub field_ident: Ident,
    /// True if `init` keyword is present (generate creation code)
    pub has_init: bool,
    /// Authority seeds for the PDA owner (from token::authority = [...] parameter)
    /// Note: Seeds should NOT include the bump - it's auto-derived or passed via `bump` parameter
    pub authority_seeds: Vec<Expr>,
    /// Mint reference (extracted from seeds or explicit parameter)
    pub mint: Option<Expr>,
    /// Owner reference (the PDA that owns this token account)
    pub owner: Option<Expr>,
    /// Optional bump seed. If None, bump is auto-derived using find_program_address.
    pub bump: Option<Expr>,
}

/// A field marked with #[light_account([init,] associated_token, ...)] (Associated Token Account).
#[derive(Clone, Debug)]
pub struct AtaField {
    pub field_ident: Ident,
    /// True if `init` keyword is present (generate creation code)
    pub has_init: bool,
    /// Owner of the ATA (from associated_token::authority = ... parameter)
    /// Note: User-facing is "authority" but maps to internal "owner" field
    pub owner: Expr,
    /// Mint for the ATA (from associated_token::mint = ... parameter)
    pub mint: Expr,
    /// Bump seed (from associated_token::bump = ...)
    pub bump: Option<Expr>,
}

// ============================================================================
// Custom Parser for #[light_account(init, [mint,] namespace::key = value, ...)]
// ============================================================================

/// Namespaced key-value pair in the attribute arguments.
/// Syntax: `namespace::key = value` (e.g., `token::mint = token_mint`)
struct NamespacedKeyValue {
    namespace: Ident,
    key: Ident,
    value: Expr,
}

impl Parse for NamespacedKeyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let namespace: Ident = input.parse()?;
        input.parse::<Token![::]>()?;
        let key: Ident = input.parse()?;

        // Check for shorthand syntax (key alone without = value)
        let value: Expr = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;

            // Handle bracketed content for authority seeds
            if key == "authority" && input.peek(syn::token::Bracket) {
                let content;
                syn::bracketed!(content in input);
                let mut elements = Vec::new();
                while !content.is_empty() {
                    let elem: Expr = content.parse()?;
                    elements.push(elem);
                    if content.peek(Token![,]) {
                        content.parse::<Token![,]>()?;
                    }
                }
                syn::parse_quote!([#(#elements),*])
            } else {
                input.parse()?
            }
        } else {
            // Shorthand: key alone means key = key
            let namespace_str = namespace.to_string();
            let key_str = key.to_string();
            if is_shorthand_key(&namespace_str, &key_str) {
                syn::parse_quote!(#key)
            } else {
                return Err(Error::new_spanned(
                    &key,
                    format!(
                        "`{}::{}` requires a value (e.g., `{}::{} = ...`)",
                        namespace_str, key_str, namespace_str, key_str
                    ),
                ));
            }
        };

        Ok(Self {
            namespace,
            key,
            value,
        })
    }
}

/// Parsed arguments from #[light_account(init, [mint,] ...)].
struct LightAccountArgs {
    /// True if `init` keyword is present (required for PDA/Mint).
    has_init: bool,
    /// True if `token` keyword is present (marks token fields - skip in LightAccounts derive).
    is_token: bool,
    /// The account type (Pda, Mint, etc.).
    account_type: LightAccountType,
    /// Namespaced key-value pairs for additional arguments.
    key_values: Vec<NamespacedKeyValue>,
}

impl Parse for LightAccountArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // First token must be `init`, `token::`, `associated_token::`, or a namespaced key
        let first: Ident = input.parse()?;

        // Handle mark-only mode: `token::key` or `associated_token::key` without `init`
        // This allows: #[light_account(token::authority = [...])]
        if input.peek(Token![::]) {
            let account_type = infer_type_from_namespace(&first)?;

            // Parse the first namespaced key-value (we already have the namespace)
            input.parse::<Token![::]>()?;
            let key: Ident = input.parse()?;

            let value = if input.peek(Token![=]) {
                input.parse::<Token![=]>()?;
                if key == "authority" && input.peek(syn::token::Bracket) {
                    let content;
                    syn::bracketed!(content in input);
                    let mut elements = Vec::new();
                    while !content.is_empty() {
                        let elem: Expr = content.parse()?;
                        elements.push(elem);
                        if content.peek(Token![,]) {
                            content.parse::<Token![,]>()?;
                        }
                    }
                    syn::parse_quote!([#(#elements),*])
                } else {
                    input.parse()?
                }
            } else {
                let key_str = key.to_string();
                let namespace_str = first.to_string();
                if is_shorthand_key(&namespace_str, &key_str) {
                    syn::parse_quote!(#key)
                } else {
                    return Err(Error::new_spanned(
                        &key,
                        format!(
                            "`{}::{}` requires a value (e.g., `{}::{} = ...`)",
                            namespace_str, key_str, namespace_str, key_str
                        ),
                    ));
                }
            };

            let mut key_values = vec![NamespacedKeyValue {
                namespace: first.clone(),
                key,
                value,
            }];

            // Parse remaining key-values
            let remaining = parse_namespaced_key_values(input, account_type)?;
            key_values.extend(remaining);

            return Ok(Self {
                has_init: false,
                is_token: true, // Skip in LightAccounts derive (for mark-only mode)
                account_type,
                key_values,
            });
        }

        // Handle old-style standalone keywords (backward compatibility)
        if first == "token" || first == "associated_token" {
            let account_type = if first == "token" {
                LightAccountType::Token
            } else {
                LightAccountType::AssociatedToken
            };
            let key_values = parse_namespaced_key_values(input, account_type)?;
            return Ok(Self {
                has_init: false,
                is_token: true,
                account_type,
                key_values,
            });
        }

        if first != "init" {
            return Err(Error::new_spanned(
                &first,
                "First argument to #[light_account] must be `init` or a namespaced key (e.g., `token::authority`)",
            ));
        }

        let mut account_type = LightAccountType::Pda;
        let mut key_values = Vec::new();

        // Parse remaining tokens
        while !input.is_empty() {
            input.parse::<Token![,]>()?;

            if input.is_empty() {
                break;
            }

            // Check if this is a namespaced key (has `::` after ident)
            if input.peek(Ident) {
                let lookahead = input.fork();
                let ident: Ident = lookahead.parse()?;

                // If followed by `::`, infer type from namespace
                if lookahead.peek(Token![::]) {
                    // Infer account type from namespace
                    let inferred_type = infer_type_from_namespace(&ident)?;

                    // If this is the first namespaced key, set account type
                    if account_type == LightAccountType::Pda {
                        account_type = inferred_type;
                    }

                    // Parse this key-value and remaining ones
                    let kv: NamespacedKeyValue = input.parse()?;
                    key_values.push(kv);

                    // Parse remaining key-values
                    let remaining = parse_namespaced_key_values(input, account_type)?;
                    key_values.extend(remaining);
                    break;
                }

                // Check for explicit type keywords (backward compatibility)
                if ident == "mint" {
                    input.parse::<Ident>()?; // consume it
                    account_type = LightAccountType::Mint;
                    key_values = parse_namespaced_key_values(input, account_type)?;
                    break;
                } else if ident == "token" {
                    input.parse::<Ident>()?; // consume it
                    account_type = LightAccountType::Token;
                    key_values = parse_namespaced_key_values(input, account_type)?;
                    break;
                } else if ident == "associated_token" {
                    input.parse::<Ident>()?; // consume it
                    account_type = LightAccountType::AssociatedToken;
                    key_values = parse_namespaced_key_values(input, account_type)?;
                    break;
                }

                // Old syntax - give helpful error
                return Err(Error::new_spanned(
                    &ident,
                    format!(
                        "Unknown keyword `{}`. Use namespaced syntax like `token::authority` or `mint::signer`",
                        ident
                    ),
                ));
            }
        }

        Ok(Self {
            has_init: true,
            is_token: false,
            account_type,
            key_values,
        })
    }
}

/// Infer account type from namespace identifier.
fn infer_type_from_namespace(namespace: &Ident) -> Result<LightAccountType, syn::Error> {
    let ns = namespace.to_string();
    match ns.as_str() {
        "token" => Ok(LightAccountType::Token),
        "associated_token" => Ok(LightAccountType::AssociatedToken),
        "mint" => Ok(LightAccountType::Mint),
        _ => Err(Error::new_spanned(
            namespace,
            format!(
                "Unknown namespace `{}`. Expected: token, associated_token, or mint",
                ns
            ),
        )),
    }
}

/// Parse namespaced key-value pairs for token, associated_token, and mint attributes.
/// Syntax: `namespace::key = value` (e.g., `token::mint = token_mint`)
fn parse_namespaced_key_values(
    input: ParseStream,
    account_type: LightAccountType,
) -> syn::Result<Vec<NamespacedKeyValue>> {
    let mut key_values = Vec::new();
    let mut seen_keys = std::collections::HashSet::new();
    let expected_namespace = account_type.namespace();

    while !input.is_empty() {
        input.parse::<Token![,]>()?;

        if input.is_empty() {
            break;
        }

        // Check if this looks like an old-style non-namespaced key
        let fork = input.fork();
        let maybe_key: Ident = fork.parse()?;

        // If followed by `=` but not `::`, it's old syntax
        if fork.peek(Token![=]) && !input.peek2(Token![::]) {
            // Check if this is just a standalone keyword
            if !is_standalone_keyword(&maybe_key.to_string()) {
                return Err(Error::new_spanned(
                    &maybe_key,
                    missing_namespace_error(&maybe_key.to_string(), expected_namespace),
                ));
            }
        }

        let kv: NamespacedKeyValue = input.parse()?;

        let namespace_str = kv.namespace.to_string();
        let key_str = kv.key.to_string();

        // Validate namespace matches account type
        if namespace_str != expected_namespace {
            return Err(Error::new_spanned(
                &kv.namespace,
                format!(
                    "Namespace `{}` doesn't match account type `{}`. Use `{}::{}` instead.",
                    namespace_str, expected_namespace, expected_namespace, key_str
                ),
            ));
        }

        // Check for duplicate keys
        if !seen_keys.insert(key_str.clone()) {
            return Err(Error::new_spanned(
                &kv.key,
                format!(
                    "Duplicate key `{}::{}` in #[light_account({}, ...)]. Each key can only appear once.",
                    namespace_str,
                    key_str,
                    expected_namespace
                ),
            ));
        }

        // Validate key is valid for this namespace
        if let Err(err_msg) = validate_namespaced_key(&namespace_str, &key_str) {
            return Err(Error::new_spanned(&kv.key, err_msg));
        }

        key_values.push(kv);
    }

    Ok(key_values)
}

// ============================================================================
// Main Parsing Function
// ============================================================================

/// Parse #[light_account(...)] attribute from a field.
/// Returns None if no light_account attribute or if it's a mark-only token/ata field.
/// Returns Some(LightAccountField) for PDA, Mint, or init Token/Ata fields.
///
/// # Arguments
/// * `field` - The field to parse
/// * `field_ident` - The field identifier
/// * `direct_proof_arg` - If `Some`, CreateAccountsProof is passed directly as an instruction arg
///   with this name, so defaults should use `<name>.field` instead of `params.create_accounts_proof.field`
pub(super) fn parse_light_account_attr(
    field: &Field,
    field_ident: &Ident,
    direct_proof_arg: &Option<Ident>,
) -> Result<Option<LightAccountField>, syn::Error> {
    for attr in &field.attrs {
        if attr.path().is_ident("light_account") {
            let args: LightAccountArgs = attr.parse_args()?;

            // Mark-only mode (token/ata without init) - handled by light_program macro
            // Return None so LightAccounts derive skips them
            // But still validate that required parameters are present
            if args.is_token && !args.has_init {
                // For mark-only token, token::authority is required but token::mint/token::owner are NOT allowed
                if args.account_type == LightAccountType::Token {
                    let has_authority = args.key_values.iter().any(|kv| kv.key == "authority");
                    if !has_authority {
                        return Err(Error::new_spanned(
                            attr,
                            "#[light_account(token, ...)] requires `token::authority = [...]` parameter",
                        ));
                    }
                    // mint and owner are only for init mode
                    for kv in &args.key_values {
                        let key = kv.key.to_string();
                        if key == "mint" || key == "owner" {
                            return Err(Error::new_spanned(
                                &kv.key,
                                format!(
                                    "`token::{}` is only allowed with `init`. \
                                     For mark-only token, use: #[light_account(token, token::authority = [...])]",
                                    key
                                ),
                            ));
                        }
                    }
                }
                return Ok(None);
            }

            // For PDA and Mint, init is required
            if !args.has_init
                && (args.account_type == LightAccountType::Pda
                    || args.account_type == LightAccountType::Mint)
            {
                return Err(Error::new_spanned(
                    attr,
                    "#[light_account] requires `init` as the first argument for PDA/Mint",
                ));
            }

            return match args.account_type {
                LightAccountType::Pda => Ok(Some(LightAccountField::Pda(Box::new(
                    build_pda_field(field, field_ident, &args.key_values, direct_proof_arg)?,
                )))),
                LightAccountType::Mint => Ok(Some(LightAccountField::Mint(Box::new(
                    build_mint_field(field_ident, &args.key_values, attr, direct_proof_arg)?,
                )))),
                LightAccountType::Token => Ok(Some(LightAccountField::TokenAccount(Box::new(
                    build_token_account_field(field_ident, &args.key_values, args.has_init, attr)?,
                )))),
                LightAccountType::AssociatedToken => {
                    Ok(Some(LightAccountField::AssociatedToken(Box::new(
                        build_ata_field(field_ident, &args.key_values, args.has_init, attr)?,
                    ))))
                }
            };
        }
    }
    Ok(None)
}

/// Build a PdaField from parsed key-value pairs.
///
/// # Arguments
/// * `direct_proof_arg` - If `Some`, use `<name>.field` for defaults instead of `params.create_accounts_proof.field`
fn build_pda_field(
    field: &Field,
    field_ident: &Ident,
    key_values: &[NamespacedKeyValue],
    direct_proof_arg: &Option<Ident>,
) -> Result<PdaField, syn::Error> {
    // Reject any key-value pairs - PDA only needs `init`
    // Tree info is always auto-fetched from CreateAccountsProof
    if !key_values.is_empty() {
        let keys: Vec<_> = key_values
            .iter()
            .map(|kv| format!("{}::{}", kv.namespace, kv.key))
            .collect();
        return Err(Error::new_spanned(
            &key_values[0].key,
            format!(
                "Unexpected arguments for PDA: {}. \
                 #[light_account(init)] takes no additional arguments. \
                 address_tree_info and output_tree are automatically sourced from CreateAccountsProof.",
                keys.join(", ")
            ),
        ));
    }

    // Always fetch from CreateAccountsProof
    let (address_tree_info, output_tree) = if let Some(proof_ident) = direct_proof_arg {
        (
            syn::parse_quote!(#proof_ident.address_tree_info),
            syn::parse_quote!(#proof_ident.output_state_tree_index),
        )
    } else {
        (
            syn::parse_quote!(params.create_accounts_proof.address_tree_info),
            syn::parse_quote!(params.create_accounts_proof.output_state_tree_index),
        )
    };

    // Validate this is an Account type (or Box<Account>)
    let (is_boxed, inner_type) = extract_account_inner_type(&field.ty).ok_or_else(|| {
        Error::new_spanned(
            &field.ty,
            "#[light_account(init)] can only be applied to Account<...> or Box<Account<...>> fields. \
             Nested Box<Box<...>> is not supported.",
        )
    })?;

    Ok(PdaField {
        ident: field_ident.clone(),
        inner_type,
        address_tree_info,
        output_tree,
        is_boxed,
    })
}

/// Build a LightMintField from parsed namespaced key-value pairs.
///
/// Mapping from new syntax to internal fields:
/// - `mint::signer` -> `mint_signer`
/// - `mint::authority` -> `authority`
/// - `mint::decimals` -> `decimals`
/// - `mint::seeds` -> `mint_seeds`
/// - `mint::bump` -> `mint_bump`
/// - `mint::freeze_authority` -> `freeze_authority`
/// - `mint::authority_seeds` -> `authority_seeds`
/// - `mint::authority_bump` -> `authority_bump`
/// - `mint::rent_payment` -> `rent_payment`
/// - `mint::write_top_up` -> `write_top_up`
/// - `mint::name` -> `name`
/// - `mint::symbol` -> `symbol`
/// - `mint::uri` -> `uri`
/// - `mint::update_authority` -> `update_authority`
/// - `mint::additional_metadata` -> `additional_metadata`
fn build_mint_field(
    field_ident: &Ident,
    key_values: &[NamespacedKeyValue],
    attr: &syn::Attribute,
    direct_proof_arg: &Option<Ident>,
) -> Result<LightMintField, syn::Error> {
    // Required fields
    let mut mint_signer: Option<Expr> = None;
    let mut authority: Option<Expr> = None;
    let mut decimals: Option<Expr> = None;
    let mut mint_seeds: Option<Expr> = None;

    // Optional fields
    let mut freeze_authority: Option<Ident> = None;
    let mut authority_seeds: Option<Expr> = None;
    let mut mint_bump: Option<Expr> = None;
    let mut authority_bump: Option<Expr> = None;
    let mut rent_payment: Option<Expr> = None;
    let mut write_top_up: Option<Expr> = None;

    // Metadata fields
    let mut name: Option<Expr> = None;
    let mut symbol: Option<Expr> = None;
    let mut uri: Option<Expr> = None;
    let mut update_authority: Option<Ident> = None;
    let mut additional_metadata: Option<Expr> = None;

    for kv in key_values {
        match kv.key.to_string().as_str() {
            // Required fields (new names)
            "signer" => mint_signer = Some(kv.value.clone()),
            "authority" => authority = Some(kv.value.clone()),
            "decimals" => decimals = Some(kv.value.clone()),
            "seeds" => mint_seeds = Some(kv.value.clone()),

            // Optional fields
            "bump" => mint_bump = Some(kv.value.clone()),
            "freeze_authority" => {
                freeze_authority = Some(expr_to_ident(&kv.value, "mint::freeze_authority")?);
            }
            "authority_seeds" => authority_seeds = Some(kv.value.clone()),
            "authority_bump" => authority_bump = Some(kv.value.clone()),
            "rent_payment" => rent_payment = Some(kv.value.clone()),
            "write_top_up" => write_top_up = Some(kv.value.clone()),

            // Metadata fields
            "name" => name = Some(kv.value.clone()),
            "symbol" => symbol = Some(kv.value.clone()),
            "uri" => uri = Some(kv.value.clone()),
            "update_authority" => {
                update_authority = Some(expr_to_ident(&kv.value, "mint::update_authority")?);
            }
            "additional_metadata" => additional_metadata = Some(kv.value.clone()),

            other => {
                return Err(Error::new_spanned(
                    &kv.key,
                    format!(
                        "Unknown key `mint::{}`. Allowed: {}",
                        other,
                        valid_keys_for_namespace("mint").join(", ")
                    ),
                ));
            }
        }
    }

    // Validate required fields
    let mint_signer = mint_signer.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `mint::signer`",
        )
    })?;
    let authority = authority.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `mint::authority`",
        )
    })?;
    let decimals = decimals.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `mint::decimals`",
        )
    })?;
    let mint_seeds = mint_seeds.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `mint::seeds`",
        )
    })?;

    // Validate metadata fields (all-or-nothing rule)
    validate_metadata_fields(
        &name,
        &symbol,
        &uri,
        &update_authority,
        &additional_metadata,
        attr,
    )?;

    // Always fetch from CreateAccountsProof - depends on whether proof is direct arg or nested
    let (address_tree_info, output_tree) = if let Some(proof_ident) = direct_proof_arg {
        (
            syn::parse_quote!(#proof_ident.address_tree_info),
            syn::parse_quote!(#proof_ident.output_state_tree_index),
        )
    } else {
        (
            syn::parse_quote!(params.create_accounts_proof.address_tree_info),
            syn::parse_quote!(params.create_accounts_proof.output_state_tree_index),
        )
    };

    Ok(LightMintField {
        field_ident: field_ident.clone(),
        mint_signer,
        authority,
        decimals,
        address_tree_info,
        output_tree,
        freeze_authority,
        mint_seeds,
        mint_bump,
        authority_seeds,
        authority_bump,
        rent_payment,
        write_top_up,
        name,
        symbol,
        uri,
        update_authority,
        additional_metadata,
    })
}

/// Build a TokenAccountField from parsed namespaced key-value pairs.
///
/// Mapping from new syntax to internal fields:
/// - `token::authority` -> `authority_seeds`
/// - `token::mint` -> `mint`
/// - `token::owner` -> `owner`
/// - `token::bump` -> `bump`
fn build_token_account_field(
    field_ident: &Ident,
    key_values: &[NamespacedKeyValue],
    has_init: bool,
    attr: &syn::Attribute,
) -> Result<TokenAccountField, syn::Error> {
    let mut authority: Option<Expr> = None;
    let mut mint: Option<Expr> = None;
    let mut owner: Option<Expr> = None;
    let mut bump: Option<Expr> = None;

    for kv in key_values {
        match kv.key.to_string().as_str() {
            "authority" => authority = Some(kv.value.clone()),
            "mint" => mint = Some(kv.value.clone()),
            "owner" => owner = Some(kv.value.clone()),
            "bump" => bump = Some(kv.value.clone()),
            other => {
                return Err(Error::new_spanned(
                    &kv.key,
                    format!(
                        "Unknown key `token::{}`. Expected: authority, mint, owner, bump",
                        other
                    ),
                ));
            }
        }
    }

    // authority is ALWAYS required (mark-only and init modes)
    if authority.is_none() {
        return Err(Error::new_spanned(
            attr,
            "#[light_account(token, ...)] requires `token::authority = [...]` parameter",
        ));
    }

    // mint and owner are required for init mode
    if has_init {
        if mint.is_none() {
            return Err(Error::new_spanned(
                attr,
                "#[light_account(init, token, ...)] requires `token::mint` parameter",
            ));
        }
        if owner.is_none() {
            return Err(Error::new_spanned(
                attr,
                "#[light_account(init, token, ...)] requires `token::owner` parameter",
            ));
        }
    }

    // Extract authority seeds from the array expression
    let authority_seeds = if let Some(ref auth_expr) = authority {
        let seeds = extract_array_elements(auth_expr)?;
        if has_init && seeds.is_empty() {
            return Err(Error::new_spanned(
                auth_expr,
                "Empty authority seeds `token::authority = []` not allowed for token account initialization. \
                 Token accounts require at least one seed to derive the PDA owner.",
            ));
        }
        seeds
    } else {
        Vec::new()
    };

    Ok(TokenAccountField {
        field_ident: field_ident.clone(),
        has_init,
        authority_seeds,
        mint,
        owner,
        bump,
    })
}

/// Build an AtaField from parsed namespaced key-value pairs.
///
/// Mapping from new syntax to internal fields:
/// - `associated_token::authority` -> `owner` (renamed to match Anchor's ATA naming)
/// - `associated_token::mint` -> `mint`
/// - `associated_token::bump` -> `bump`
fn build_ata_field(
    field_ident: &Ident,
    key_values: &[NamespacedKeyValue],
    has_init: bool,
    attr: &syn::Attribute,
) -> Result<AtaField, syn::Error> {
    let mut owner: Option<Expr> = None; // from associated_token::authority
    let mut mint: Option<Expr> = None;
    let mut bump: Option<Expr> = None;

    for kv in key_values {
        match kv.key.to_string().as_str() {
            "authority" => owner = Some(kv.value.clone()), // authority -> owner
            "mint" => mint = Some(kv.value.clone()),
            "bump" => bump = Some(kv.value.clone()),
            other => {
                return Err(Error::new_spanned(
                    &kv.key,
                    format!(
                        "Unknown key `associated_token::{}`. Allowed: authority, mint, bump",
                        other
                    ),
                ));
            }
        }
    }

    // Validate required fields
    let owner = owner.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account([init,] associated_token, ...)] requires `associated_token::authority` parameter",
        )
    })?;
    let mint = mint.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account([init,] associated_token, ...)] requires `associated_token::mint` parameter",
        )
    })?;

    Ok(AtaField {
        field_ident: field_ident.clone(),
        has_init,
        owner,
        mint,
        bump,
    })
}

/// Convert an expression to an identifier (for field references).
fn expr_to_ident(expr: &Expr, field_name: &str) -> Result<Ident, syn::Error> {
    match expr {
        Expr::Path(path) => path.path.get_ident().cloned().ok_or_else(|| {
            Error::new_spanned(expr, format!("`{field_name}` must be a simple identifier"))
        }),
        _ => Err(Error::new_spanned(
            expr,
            format!("`{field_name}` must be a simple identifier"),
        )),
    }
}

/// Extract elements from an array expression.
fn extract_array_elements(expr: &Expr) -> Result<Vec<Expr>, syn::Error> {
    match expr {
        Expr::Array(arr) => Ok(arr.elems.iter().cloned().collect()),
        Expr::Reference(r) => extract_array_elements(&r.expr),
        _ => Err(Error::new_spanned(
            expr,
            "Expected array expression like `[b\"seed\", other.key()]`",
        )),
    }
}

/// Validates TokenMetadata field requirements.
///
/// Rules:
/// 1. `name`, `symbol`, `uri` must all be defined together or none
/// 2. `update_authority` and `additional_metadata` require `name`, `symbol`, `uri`
fn validate_metadata_fields(
    name: &Option<Expr>,
    symbol: &Option<Expr>,
    uri: &Option<Expr>,
    update_authority: &Option<Ident>,
    additional_metadata: &Option<Expr>,
    attr: &syn::Attribute,
) -> Result<(), syn::Error> {
    let has_name = name.is_some();
    let has_symbol = symbol.is_some();
    let has_uri = uri.is_some();
    let has_update_authority = update_authority.is_some();
    let has_additional_metadata = additional_metadata.is_some();

    let core_metadata_count = [has_name, has_symbol, has_uri]
        .iter()
        .filter(|&&x| x)
        .count();

    // Rule 1: name, symbol, uri must all be defined together or none
    if core_metadata_count > 0 && core_metadata_count < 3 {
        return Err(Error::new_spanned(
            attr,
            "TokenMetadata requires all of `mint::name`, `mint::symbol`, and `mint::uri` to be specified together",
        ));
    }

    // Rule 2: update_authority and additional_metadata require name, symbol, uri
    if (has_update_authority || has_additional_metadata) && core_metadata_count == 0 {
        return Err(Error::new_spanned(
            attr,
            "`mint::update_authority` and `mint::additional_metadata` require `mint::name`, `mint::symbol`, and `mint::uri` to also be specified",
        ));
    }

    Ok(())
}

// ============================================================================
// Conversion to existing types (for compatibility with existing code gen)
// ============================================================================

/// Convert PdaField to ParsedPdaField (used by existing codegen).
impl From<PdaField> for super::parse::ParsedPdaField {
    fn from(pda: PdaField) -> Self {
        Self {
            ident: pda.ident,
            inner_type: pda.inner_type,
            address_tree_info: pda.address_tree_info,
            output_tree: pda.output_tree,
            is_boxed: pda.is_boxed,
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_parse_light_account_pda_bare() {
        let field: syn::Field = parse_quote! {
            #[light_account(init)]
            pub record: Account<'info, MyRecord>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::Pda(pda) => {
                assert_eq!(pda.ident.to_string(), "record");
                assert!(!pda.is_boxed);
            }
            _ => panic!("Expected PDA field"),
        }
    }

    #[test]
    fn test_parse_pda_tree_keywords_rejected() {
        // Tree keywords are no longer allowed - they're auto-fetched from CreateAccountsProof
        let field: syn::Field = parse_quote! {
            #[light_account(init, pda::address_tree_info = custom_tree)]
            pub record: Account<'info, MyRecord>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_light_account_mint() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint,
                mint::signer = mint_signer,
                mint::authority = authority,
                mint::decimals = 9,
                mint::seeds = &[b"test"]
            )]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::Mint(mint) => {
                assert_eq!(mint.field_ident.to_string(), "cmint");
            }
            _ => panic!("Expected Mint field"),
        }
    }

    #[test]
    fn test_parse_light_account_mint_with_metadata() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint,
                mint::signer = mint_signer,
                mint::authority = authority,
                mint::decimals = 9,
                mint::seeds = &[b"test"],
                mint::name = params.name.clone(),
                mint::symbol = params.symbol.clone(),
                mint::uri = params.uri.clone()
            )]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::Mint(mint) => {
                assert!(mint.name.is_some());
                assert!(mint.symbol.is_some());
                assert!(mint.uri.is_some());
            }
            _ => panic!("Expected Mint field"),
        }
    }

    #[test]
    fn test_parse_light_account_missing_init() {
        let field: syn::Field = parse_quote! {
            #[light_account(mint, mint::decimals = 9)]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_light_account_mint_missing_required() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint, mint::decimals = 9)]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_light_account_partial_metadata_fails() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint,
                mint::signer = mint_signer,
                mint::authority = authority,
                mint::decimals = 9,
                mint::seeds = &[b"test"],
                mint::name = params.name.clone()
            )]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_light_account_attr_returns_none() {
        let field: syn::Field = parse_quote! {
            pub record: Account<'info, MyRecord>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // ========================================================================
    // Token Account Tests
    // ========================================================================

    #[test]
    fn test_parse_token_mark_only_returns_none() {
        // Mark-only mode (no init) should return None for LightAccounts derive
        let field: syn::Field = parse_quote! {
            #[light_account(token, token::authority = [b"authority"])]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_token_init_creates_field() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, token, token::authority = [b"authority"], token::mint = token_mint, token::owner = vault_authority)]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::TokenAccount(token) => {
                assert_eq!(token.field_ident.to_string(), "vault");
                assert!(token.has_init);
                assert!(!token.authority_seeds.is_empty());
                assert!(token.mint.is_some());
                assert!(token.owner.is_some());
            }
            _ => panic!("Expected TokenAccount field"),
        }
    }

    #[test]
    fn test_parse_token_init_missing_authority_fails() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, token)]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("authority"));
    }

    #[test]
    fn test_parse_token_mark_only_missing_authority_fails() {
        // Mark-only token now requires authority
        let field: syn::Field = parse_quote! {
            #[light_account(token)]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("authority"),
            "Expected error about missing authority, got: {}",
            err
        );
    }

    #[test]
    fn test_parse_token_mark_only_rejects_mint() {
        // Mark-only token should not allow mint parameter
        let field: syn::Field = parse_quote! {
            #[light_account(token, token::authority = [b"auth"], token::mint = token_mint)]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("mint") && err.contains("only allowed with `init`"),
            "Expected error about mint only for init, got: {}",
            err
        );
    }

    #[test]
    fn test_parse_token_mark_only_rejects_owner() {
        // Mark-only token should not allow owner parameter
        let field: syn::Field = parse_quote! {
            #[light_account(token, token::authority = [b"auth"], token::owner = vault_authority)]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("owner") && err.contains("only allowed with `init`"),
            "Expected error about owner only for init, got: {}",
            err
        );
    }

    #[test]
    fn test_parse_token_init_missing_mint_fails() {
        // Token init requires mint parameter
        let field: syn::Field = parse_quote! {
            #[light_account(init, token, token::authority = [b"authority"], token::owner = vault_authority)]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("mint"),
            "Expected error about missing mint, got: {}",
            err
        );
    }

    #[test]
    fn test_parse_token_init_missing_owner_fails() {
        // Token init requires owner parameter
        let field: syn::Field = parse_quote! {
            #[light_account(init, token, token::authority = [b"authority"], token::mint = token_mint)]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("owner"),
            "Expected error about missing owner, got: {}",
            err
        );
    }

    // ========================================================================
    // Associated Token Tests
    // ========================================================================

    #[test]
    fn test_parse_associated_token_mark_only_returns_none() {
        // Mark-only mode (no init) should return None for LightAccounts derive
        let field: syn::Field = parse_quote! {
            #[light_account(associated_token, associated_token::authority = owner, associated_token::mint = mint)]
            pub user_ata: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_associated_token_init_creates_field() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, associated_token, associated_token::authority = owner, associated_token::mint = mint)]
            pub user_ata: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::AssociatedToken(ata) => {
                assert_eq!(ata.field_ident.to_string(), "user_ata");
                assert!(ata.has_init);
            }
            _ => panic!("Expected AssociatedToken field"),
        }
    }

    #[test]
    fn test_parse_associated_token_init_missing_authority_fails() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, associated_token, associated_token::mint = mint)]
            pub user_ata: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("authority"));
    }

    #[test]
    fn test_parse_associated_token_init_missing_mint_fails() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, associated_token, associated_token::authority = owner)]
            pub user_ata: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("mint"));
    }

    #[test]
    fn test_parse_token_unknown_argument_fails() {
        let field: syn::Field = parse_quote! {
            #[light_account(token, token::authority = [b"auth"], token::unknown = foo)]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("unknown"));
    }

    #[test]
    fn test_parse_associated_token_unknown_argument_fails() {
        let field: syn::Field = parse_quote! {
            #[light_account(associated_token, associated_token::authority = owner, associated_token::mint = mint, associated_token::unknown = foo)]
            pub user_ata: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("unknown"));
    }

    #[test]
    fn test_parse_associated_token_shorthand_syntax() {
        // Test shorthand syntax: mint, authority, bump without = value
        let field: syn::Field = parse_quote! {
            #[light_account(init, associated_token, associated_token::authority, associated_token::mint, associated_token::bump)]
            pub user_ata: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::AssociatedToken(ata) => {
                assert_eq!(ata.field_ident.to_string(), "user_ata");
                assert!(ata.has_init);
                assert!(ata.bump.is_some());
            }
            _ => panic!("Expected AssociatedToken field"),
        }
    }

    #[test]
    fn test_parse_token_duplicate_key_fails() {
        // Duplicate keys should be rejected
        let field: syn::Field = parse_quote! {
            #[light_account(token, token::authority = [b"auth1"], token::authority = [b"auth2"])]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("Duplicate key"),
            "Expected error about duplicate key, got: {}",
            err
        );
    }

    #[test]
    fn test_parse_associated_token_duplicate_key_fails() {
        // Duplicate keys in associated_token should also be rejected
        let field: syn::Field = parse_quote! {
            #[light_account(init, associated_token, associated_token::authority = foo, associated_token::authority = bar, associated_token::mint)]
            pub user_ata: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("Duplicate key"),
            "Expected error about duplicate key, got: {}",
            err
        );
    }

    #[test]
    fn test_parse_token_init_empty_authority_fails() {
        // Empty authority seeds with init should be rejected
        let field: syn::Field = parse_quote! {
            #[light_account(init, token, token::authority = [], token::mint = token_mint, token::owner = vault_authority)]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("Empty authority seeds"),
            "Expected error about empty authority seeds, got: {}",
            err
        );
    }

    #[test]
    fn test_parse_token_non_init_empty_authority_allowed() {
        // Empty authority seeds without init should be allowed (mark-only mode)
        let field: syn::Field = parse_quote! {
            #[light_account(token, token::authority = [])]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        // Mark-only mode returns Ok(None)
        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_pda_with_direct_proof_arg_uses_proof_ident_for_defaults() {
        // When CreateAccountsProof is passed as a direct instruction arg (not nested in params),
        // the default address_tree_info and output_tree should reference the proof arg directly.
        let field: syn::Field = parse_quote! {
            #[light_account(init)]
            pub record: Account<'info, MyRecord>
        };
        let field_ident = field.ident.clone().unwrap();

        // Simulate passing CreateAccountsProof as direct arg named "proof"
        let proof_ident: Ident = parse_quote!(proof);
        let direct_proof_arg = Some(proof_ident.clone());

        let result = parse_light_account_attr(&field, &field_ident, &direct_proof_arg);
        assert!(
            result.is_ok(),
            "Should parse successfully with direct proof arg"
        );
        let result = result.unwrap();
        assert!(result.is_some(), "Should return Some for init PDA");

        match result.unwrap() {
            LightAccountField::Pda(pda) => {
                assert_eq!(pda.ident.to_string(), "record");

                // Verify defaults use the direct proof identifier
                // address_tree_info should be: proof.address_tree_info
                let addr_tree_info = &pda.address_tree_info;
                let addr_tree_str = quote::quote!(#addr_tree_info).to_string();
                assert!(
                    addr_tree_str.contains("proof"),
                    "address_tree_info should reference 'proof', got: {}",
                    addr_tree_str
                );
                assert!(
                    addr_tree_str.contains("address_tree_info"),
                    "address_tree_info should access .address_tree_info field, got: {}",
                    addr_tree_str
                );

                // output_tree should be: proof.output_state_tree_index
                let output_tree = &pda.output_tree;
                let output_tree_str = quote::quote!(#output_tree).to_string();
                assert!(
                    output_tree_str.contains("proof"),
                    "output_tree should reference 'proof', got: {}",
                    output_tree_str
                );
                assert!(
                    output_tree_str.contains("output_state_tree_index"),
                    "output_tree should access .output_state_tree_index field, got: {}",
                    output_tree_str
                );
            }
            _ => panic!("Expected PDA field"),
        }
    }

    #[test]
    fn test_parse_mint_with_direct_proof_arg_uses_proof_ident_for_defaults() {
        // When CreateAccountsProof is passed as a direct instruction arg,
        // the default address_tree_info should reference the proof arg directly.
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint,
                mint::signer = mint_signer,
                mint::authority = authority,
                mint::decimals = 9,
                mint::seeds = &[b"test"]
            )]
            pub cmint: UncheckedAccount<'info>
        };
        let field_ident = field.ident.clone().unwrap();

        // Simulate passing CreateAccountsProof as direct arg named "create_proof"
        let proof_ident: Ident = parse_quote!(create_proof);
        let direct_proof_arg = Some(proof_ident.clone());

        let result = parse_light_account_attr(&field, &field_ident, &direct_proof_arg);
        assert!(
            result.is_ok(),
            "Should parse successfully with direct proof arg"
        );
        let result = result.unwrap();
        assert!(result.is_some(), "Should return Some for init mint");

        match result.unwrap() {
            LightAccountField::Mint(mint) => {
                assert_eq!(mint.field_ident.to_string(), "cmint");

                // Verify default address_tree_info uses the direct proof identifier
                // Should be: create_proof.address_tree_info
                let addr_tree_info = &mint.address_tree_info;
                let addr_tree_str = quote::quote!(#addr_tree_info).to_string();
                assert!(
                    addr_tree_str.contains("create_proof"),
                    "address_tree_info should reference 'create_proof', got: {}",
                    addr_tree_str
                );
                assert!(
                    addr_tree_str.contains("address_tree_info"),
                    "address_tree_info should access .address_tree_info field, got: {}",
                    addr_tree_str
                );

                // Verify default output_tree uses the direct proof identifier
                // Should be: create_proof.output_state_tree_index
                let output_tree = &mint.output_tree;
                let output_tree_str = quote::quote!(#output_tree).to_string();
                assert!(
                    output_tree_str.contains("create_proof"),
                    "output_tree should reference 'create_proof', got: {}",
                    output_tree_str
                );
                assert!(
                    output_tree_str.contains("output_state_tree_index"),
                    "output_tree should access .output_state_tree_index field, got: {}",
                    output_tree_str
                );
            }
            _ => panic!("Expected Mint field"),
        }
    }

    // ========================================================================
    // Bump Parameter Tests
    // ========================================================================

    #[test]
    fn test_parse_token_with_bump_parameter() {
        // Test token with explicit bump parameter
        let field: syn::Field = parse_quote! {
            #[light_account(init, token,
                token::authority = [b"vault", self.offer.key()],
                token::mint = token_mint,
                token::owner = vault_authority,
                token::bump = params.vault_bump
            )]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(
            result.is_ok(),
            "Should parse successfully with bump parameter"
        );
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::TokenAccount(token) => {
                assert_eq!(token.field_ident.to_string(), "vault");
                assert!(token.has_init);
                assert!(!token.authority_seeds.is_empty());
                assert!(token.bump.is_some(), "bump should be Some when provided");
            }
            _ => panic!("Expected TokenAccount field"),
        }
    }

    #[test]
    fn test_parse_token_without_bump_backwards_compatible() {
        // Test token without bump (backwards compatible - bump will be auto-derived)
        let field: syn::Field = parse_quote! {
            #[light_account(init, token,
                token::authority = [b"vault", self.offer.key()],
                token::mint = token_mint,
                token::owner = vault_authority
            )]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(
            result.is_ok(),
            "Should parse successfully without bump parameter"
        );
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::TokenAccount(token) => {
                assert_eq!(token.field_ident.to_string(), "vault");
                assert!(token.has_init);
                assert!(!token.authority_seeds.is_empty());
                assert!(
                    token.bump.is_none(),
                    "bump should be None when not provided"
                );
            }
            _ => panic!("Expected TokenAccount field"),
        }
    }

    #[test]
    fn test_parse_mint_with_mint_bump() {
        // Test mint with explicit mint::bump parameter
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint,
                mint::signer = mint_signer,
                mint::authority = authority,
                mint::decimals = 9,
                mint::seeds = &[b"mint"],
                mint::bump = params.mint_bump
            )]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(
            result.is_ok(),
            "Should parse successfully with mint::bump parameter"
        );
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::Mint(mint) => {
                assert_eq!(mint.field_ident.to_string(), "cmint");
                assert!(
                    mint.mint_bump.is_some(),
                    "mint_bump should be Some when provided"
                );
            }
            _ => panic!("Expected Mint field"),
        }
    }

    #[test]
    fn test_parse_mint_with_authority_bump() {
        // Test mint with authority_seeds and authority_bump
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint,
                mint::signer = mint_signer,
                mint::authority = authority,
                mint::decimals = 9,
                mint::seeds = &[b"mint"],
                mint::authority_seeds = &[b"auth"],
                mint::authority_bump = params.auth_bump
            )]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(
            result.is_ok(),
            "Should parse successfully with authority_bump parameter"
        );
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::Mint(mint) => {
                assert_eq!(mint.field_ident.to_string(), "cmint");
                assert!(
                    mint.authority_seeds.is_some(),
                    "authority_seeds should be Some"
                );
                assert!(
                    mint.authority_bump.is_some(),
                    "authority_bump should be Some when provided"
                );
            }
            _ => panic!("Expected Mint field"),
        }
    }

    #[test]
    fn test_parse_mint_without_bumps_backwards_compatible() {
        // Test mint without bump parameters (backwards compatible - bumps will be auto-derived)
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint,
                mint::signer = mint_signer,
                mint::authority = authority,
                mint::decimals = 9,
                mint::seeds = &[b"mint"],
                mint::authority_seeds = &[b"auth"]
            )]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(
            result.is_ok(),
            "Should parse successfully without bump parameters"
        );
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::Mint(mint) => {
                assert_eq!(mint.field_ident.to_string(), "cmint");
                assert!(
                    mint.mint_bump.is_none(),
                    "mint_bump should be None when not provided"
                );
                assert!(
                    mint.authority_seeds.is_some(),
                    "authority_seeds should be Some"
                );
                assert!(
                    mint.authority_bump.is_none(),
                    "authority_bump should be None when not provided"
                );
            }
            _ => panic!("Expected Mint field"),
        }
    }

    #[test]
    fn test_parse_token_bump_shorthand_syntax() {
        // Test token with bump shorthand syntax (token::bump = bump)
        let field: syn::Field = parse_quote! {
            #[light_account(init, token,
                token::authority = [b"vault"],
                token::mint = token_mint,
                token::owner = vault_authority,
                token::bump
            )]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(
            result.is_ok(),
            "Should parse successfully with bump shorthand"
        );
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::TokenAccount(token) => {
                assert!(
                    token.bump.is_some(),
                    "bump should be Some with shorthand syntax"
                );
            }
            _ => panic!("Expected TokenAccount field"),
        }
    }

    // ========================================================================
    // Namespace Validation Tests
    // ========================================================================

    #[test]
    fn test_parse_wrong_namespace_fails() {
        // Using mint:: namespace with token account type should fail
        let field: syn::Field = parse_quote! {
            #[light_account(token, mint::authority = [b"auth"])]
            pub vault: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("doesn't match account type"),
            "Expected namespace mismatch error, got: {}",
            err
        );
    }

    #[test]
    fn test_old_syntax_gives_helpful_error() {
        // Old syntax without namespace should give helpful migration error
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint, authority = some_authority)]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("Missing namespace prefix") || err.contains("mint::authority"),
            "Expected helpful migration error, got: {}",
            err
        );
    }
}
