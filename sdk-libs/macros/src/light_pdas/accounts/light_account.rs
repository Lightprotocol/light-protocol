//! Unified #[light_account(...)] attribute parsing.
//!
//! This module provides a single unified syntax for all Light Protocol account types:
//! - `#[light_account(init)]` - PDAs
//! - `#[light_account(init, mint, ...)]` - Light Mints
//! - `#[light_account(token, ...)]` - Light token accounts
//!
//! Note: Token fields are NOT processed here - they're handled by seed_extraction.rs
//! in the light_program macro. This parser returns None for token fields.

use syn::{
    parse::{Parse, ParseStream},
    Error, Expr, Field, Ident, Token, Type,
};

use super::mint::LightMintField;
pub(super) use crate::light_pdas::account::seed_extraction::extract_account_inner_type;

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
    /// Authority seeds for the PDA owner (from authority = [...] parameter)
    pub authority_seeds: Vec<Expr>,
    /// Mint reference (extracted from seeds or explicit parameter)
    pub mint: Option<Expr>,
    /// Owner reference (the PDA that owns this token account)
    pub owner: Option<Expr>,
}

/// A field marked with #[light_account([init,] ata, ...)] (Associated Token Account).
#[derive(Clone, Debug)]
pub struct AtaField {
    pub field_ident: Ident,
    /// True if `init` keyword is present (generate creation code)
    pub has_init: bool,
    /// Owner of the ATA (from owner = ... parameter)
    pub owner: Expr,
    /// Mint for the ATA (from mint = ... parameter)
    pub mint: Expr,
    /// Bump seed (from #[account(seeds = [...], bump)])
    pub bump: Option<Expr>,
}

// ============================================================================
// Custom Parser for #[light_account(init, [mint,] key = value, ...)]
// ============================================================================

/// Key-value pair in the attribute arguments.
struct KeyValue {
    key: Ident,
    value: Expr,
}

impl Parse for KeyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        Ok(Self { key, value })
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
    /// Key-value pairs for additional arguments.
    key_values: Vec<KeyValue>,
}

impl Parse for LightAccountArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // First token must be `init`, `token`, or `associated_token`
        let first: Ident = input.parse()?;

        // Handle `token` or `associated_token` as first argument (mark-only mode, no init)
        if first == "token" || first == "associated_token" {
            let account_type = if first == "token" {
                LightAccountType::Token
            } else {
                LightAccountType::AssociatedToken
            };
            let key_values = parse_token_ata_key_values(input, &first)?;
            return Ok(Self {
                has_init: false,
                is_token: true, // Skip in LightAccounts derive (for mark-only mode)
                account_type,
                key_values,
            });
        }

        if first != "init" {
            return Err(Error::new_spanned(
                &first,
                "First argument to #[light_account] must be `init`, `token`, or `associated_token`",
            ));
        }

        let mut account_type = LightAccountType::Pda;
        let mut key_values = Vec::new();

        // Parse remaining tokens
        while !input.is_empty() {
            input.parse::<Token![,]>()?;

            // Check if this is a type keyword (mint, token, associated_token)
            if input.peek(Ident) {
                let lookahead = input.fork();
                let ident: Ident = lookahead.parse()?;

                // Check for type keywords (not followed by `=`)
                if !lookahead.peek(Token![=]) {
                    if ident == "mint" {
                        input.parse::<Ident>()?; // consume it
                        account_type = LightAccountType::Mint;
                        continue;
                    } else if ident == "token" {
                        input.parse::<Ident>()?; // consume it
                        account_type = LightAccountType::Token;
                        // Parse remaining token-specific key-values
                        key_values = parse_token_ata_key_values(input, &ident)?;
                        break;
                    } else if ident == "associated_token" {
                        input.parse::<Ident>()?; // consume it
                        account_type = LightAccountType::AssociatedToken;
                        // Parse remaining associated_token-specific key-values
                        key_values = parse_token_ata_key_values(input, &ident)?;
                        break;
                    }
                }
            }

            // Otherwise it's a key-value pair
            if !input.is_empty() {
                let kv: KeyValue = input.parse()?;
                key_values.push(kv);
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

/// Parse key-value pairs for token and associated_token attributes.
/// Handles both bracketed arrays (authority = [...]) and simple values (owner = ident).
/// Supports shorthand syntax for mint, owner, bump (e.g., `mint` alone means `mint = mint`).
fn parse_token_ata_key_values(
    input: ParseStream,
    account_type_name: &Ident,
) -> syn::Result<Vec<KeyValue>> {
    let mut key_values = Vec::new();
    let mut seen_keys = std::collections::HashSet::new();
    let valid_keys = if account_type_name == "token" {
        &["authority", "mint", "owner"][..]
    } else {
        // associated_token
        &["owner", "mint", "bump"][..]
    };

    while !input.is_empty() {
        input.parse::<Token![,]>()?;

        if input.is_empty() {
            break;
        }

        let key: Ident = input.parse()?;
        let key_str = key.to_string();

        // Check for duplicate keys
        if !seen_keys.insert(key_str.clone()) {
            return Err(Error::new_spanned(
                &key,
                format!(
                    "Duplicate key `{}` in #[light_account({}, ...)]. Each key can only appear once.",
                    key_str,
                    account_type_name
                ),
            ));
        }

        if !valid_keys.contains(&key_str.as_str()) {
            return Err(Error::new_spanned(
                &key,
                format!(
                    "Unknown argument `{}` in #[light_account({}, ...)]. \
                     Allowed: {}",
                    key,
                    account_type_name,
                    valid_keys.join(", ")
                ),
            ));
        }

        // Check for shorthand syntax (key alone without =) for mint, owner, bump
        let value: Expr = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;

            // Handle bracketed content for authority seeds
            if key == "authority" && input.peek(syn::token::Bracket) {
                let content;
                syn::bracketed!(content in input);
                // Parse as array expression
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
            // Shorthand: key alone means key = key (for mint, owner, bump)
            if key_str == "mint" || key_str == "owner" || key_str == "bump" {
                syn::parse_quote!(#key)
            } else {
                return Err(Error::new_spanned(
                    &key,
                    format!("`{}` requires a value (e.g., `{} = ...`)", key_str, key_str),
                ));
            }
        };

        key_values.push(KeyValue { key, value });
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
            if args.is_token && !args.has_init {
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
    key_values: &[KeyValue],
    direct_proof_arg: &Option<Ident>,
) -> Result<PdaField, syn::Error> {
    let mut address_tree_info: Option<Expr> = None;
    let mut output_tree: Option<Expr> = None;

    for kv in key_values {
        match kv.key.to_string().as_str() {
            "address_tree_info" => address_tree_info = Some(kv.value.clone()),
            "output_tree" => output_tree = Some(kv.value.clone()),
            other => {
                return Err(Error::new_spanned(
                    &kv.key,
                    format!(
                        "Unknown argument `{other}` for PDA. Expected: address_tree_info, output_tree"
                    ),
                ));
            }
        }
    }

    // Use defaults if not specified - depends on whether CreateAccountsProof is direct arg or nested
    let address_tree_info = address_tree_info.unwrap_or_else(|| {
        if let Some(proof_ident) = direct_proof_arg {
            syn::parse_quote!(#proof_ident.address_tree_info)
        } else {
            syn::parse_quote!(params.create_accounts_proof.address_tree_info)
        }
    });
    let output_tree = output_tree.unwrap_or_else(|| {
        if let Some(proof_ident) = direct_proof_arg {
            syn::parse_quote!(#proof_ident.output_state_tree_index)
        } else {
            syn::parse_quote!(params.create_accounts_proof.output_state_tree_index)
        }
    });

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

/// Build a LightMintField from parsed key-value pairs.
///
/// # Arguments
/// * `direct_proof_arg` - If `Some`, use `<name>.field` for defaults instead of `params.create_accounts_proof.field`
fn build_mint_field(
    field_ident: &Ident,
    key_values: &[KeyValue],
    attr: &syn::Attribute,
    direct_proof_arg: &Option<Ident>,
) -> Result<LightMintField, syn::Error> {
    // Required fields
    let mut mint_signer: Option<Expr> = None;
    let mut authority: Option<Expr> = None;
    let mut decimals: Option<Expr> = None;
    let mut mint_seeds: Option<Expr> = None;

    // Optional fields
    let mut address_tree_info: Option<Expr> = None;
    let mut freeze_authority: Option<Ident> = None;
    let mut authority_seeds: Option<Expr> = None;
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
            "mint_signer" => mint_signer = Some(kv.value.clone()),
            "authority" => authority = Some(kv.value.clone()),
            "decimals" => decimals = Some(kv.value.clone()),
            "mint_seeds" => mint_seeds = Some(kv.value.clone()),
            "address_tree_info" => address_tree_info = Some(kv.value.clone()),
            "freeze_authority" => {
                freeze_authority = Some(expr_to_ident(&kv.value, "freeze_authority")?);
            }
            "authority_seeds" => authority_seeds = Some(kv.value.clone()),
            "rent_payment" => rent_payment = Some(kv.value.clone()),
            "write_top_up" => write_top_up = Some(kv.value.clone()),
            "name" => name = Some(kv.value.clone()),
            "symbol" => symbol = Some(kv.value.clone()),
            "uri" => uri = Some(kv.value.clone()),
            "update_authority" => {
                update_authority = Some(expr_to_ident(&kv.value, "update_authority")?);
            }
            "additional_metadata" => additional_metadata = Some(kv.value.clone()),
            other => {
                return Err(Error::new_spanned(
                    &kv.key,
                    format!("Unknown argument `{other}` for mint"),
                ));
            }
        }
    }

    // Validate required fields
    let mint_signer = mint_signer.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `mint_signer`",
        )
    })?;
    let authority = authority.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `authority`",
        )
    })?;
    let decimals = decimals.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `decimals`",
        )
    })?;
    let mint_seeds = mint_seeds.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account(init, mint, ...)] requires `mint_seeds`",
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

    // address_tree_info defaults - depends on whether CreateAccountsProof is direct arg or nested
    let address_tree_info = address_tree_info.unwrap_or_else(|| {
        if let Some(proof_ident) = direct_proof_arg {
            syn::parse_quote!(#proof_ident.address_tree_info)
        } else {
            syn::parse_quote!(params.create_accounts_proof.address_tree_info)
        }
    });

    Ok(LightMintField {
        field_ident: field_ident.clone(),
        mint_signer,
        authority,
        decimals,
        address_tree_info,
        freeze_authority,
        mint_seeds,
        authority_seeds,
        rent_payment,
        write_top_up,
        name,
        symbol,
        uri,
        update_authority,
        additional_metadata,
    })
}

/// Build a TokenAccountField from parsed key-value pairs.
fn build_token_account_field(
    field_ident: &Ident,
    key_values: &[KeyValue],
    has_init: bool,
    attr: &syn::Attribute,
) -> Result<TokenAccountField, syn::Error> {
    let mut authority: Option<Expr> = None;
    let mut mint: Option<Expr> = None;
    let mut owner: Option<Expr> = None;

    for kv in key_values {
        match kv.key.to_string().as_str() {
            "authority" => authority = Some(kv.value.clone()),
            "mint" => mint = Some(kv.value.clone()),
            "owner" => owner = Some(kv.value.clone()),
            other => {
                return Err(Error::new_spanned(
                    &kv.key,
                    format!(
                        "Unknown argument `{other}` for token. \
                         Expected: authority, mint, owner"
                    ),
                ));
            }
        }
    }

    // Validate required fields for init mode
    if has_init && authority.is_none() {
        return Err(Error::new_spanned(
            attr,
            "#[light_account(init, token, ...)] requires `authority = [...]` parameter",
        ));
    }

    // Extract authority seeds from the array expression
    let authority_seeds = if let Some(ref auth_expr) = authority {
        let seeds = extract_array_elements(auth_expr)?;
        if has_init && seeds.is_empty() {
            return Err(Error::new_spanned(
                auth_expr,
                "Empty authority seeds `authority = []` not allowed for token account initialization. \
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
    })
}

/// Build an AtaField from parsed key-value pairs.
fn build_ata_field(
    field_ident: &Ident,
    key_values: &[KeyValue],
    has_init: bool,
    attr: &syn::Attribute,
) -> Result<AtaField, syn::Error> {
    let mut owner: Option<Expr> = None;
    let mut mint: Option<Expr> = None;
    let mut bump: Option<Expr> = None;

    for kv in key_values {
        match kv.key.to_string().as_str() {
            "owner" => owner = Some(kv.value.clone()),
            "mint" => mint = Some(kv.value.clone()),
            "bump" => bump = Some(kv.value.clone()),
            other => {
                return Err(Error::new_spanned(
                    &kv.key,
                    format!(
                        "Unknown argument `{other}` in #[light_account(associated_token, ...)]. \
                         Allowed: owner, mint, bump"
                    ),
                ));
            }
        }
    }

    // Validate required fields
    let owner = owner.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account([init,] associated_token, ...)] requires `owner` parameter",
        )
    })?;
    let mint = mint.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "#[light_account([init,] associated_token, ...)] requires `mint` parameter",
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
            "TokenMetadata requires all of `name`, `symbol`, and `uri` to be specified together",
        ));
    }

    // Rule 2: update_authority and additional_metadata require name, symbol, uri
    if (has_update_authority || has_additional_metadata) && core_metadata_count == 0 {
        return Err(Error::new_spanned(
            attr,
            "`update_authority` and `additional_metadata` require `name`, `symbol`, and `uri` to also be specified",
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
    fn test_parse_light_account_pda_with_options() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, address_tree_info = custom_tree, output_tree = custom_output)]
            pub record: Account<'info, MyRecord>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            LightAccountField::Pda(_) => {}
            _ => panic!("Expected PDA field"),
        }
    }

    #[test]
    fn test_parse_light_account_mint() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint,
                mint_signer = mint_signer,
                authority = authority,
                decimals = 9,
                mint_seeds = &[b"test"]
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
                mint_signer = mint_signer,
                authority = authority,
                decimals = 9,
                mint_seeds = &[b"test"],
                name = params.name.clone(),
                symbol = params.symbol.clone(),
                uri = params.uri.clone()
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
            #[light_account(mint, decimals = 9)]
            pub cmint: UncheckedAccount<'info>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_light_account_mint_missing_required() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, mint, decimals = 9)]
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
                mint_signer = mint_signer,
                authority = authority,
                decimals = 9,
                mint_seeds = &[b"test"],
                name = params.name.clone()
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
            #[light_account(token, authority = [b"authority"])]
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
            #[light_account(init, token, authority = [b"authority"])]
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

    // ========================================================================
    // Associated Token Tests
    // ========================================================================

    #[test]
    fn test_parse_associated_token_mark_only_returns_none() {
        // Mark-only mode (no init) should return None for LightAccounts derive
        let field: syn::Field = parse_quote! {
            #[light_account(associated_token, owner = owner, mint = mint)]
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
            #[light_account(init, associated_token, owner = owner, mint = mint)]
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
    fn test_parse_associated_token_init_missing_owner_fails() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, associated_token, mint = mint)]
            pub user_ata: Account<'info, CToken>
        };
        let ident = field.ident.clone().unwrap();

        let result = parse_light_account_attr(&field, &ident, &None);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("owner"));
    }

    #[test]
    fn test_parse_associated_token_init_missing_mint_fails() {
        let field: syn::Field = parse_quote! {
            #[light_account(init, associated_token, owner = owner)]
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
            #[light_account(token, authority = [b"auth"], unknown = foo)]
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
            #[light_account(associated_token, owner = owner, mint = mint, unknown = foo)]
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
        // Test shorthand syntax: mint, owner, bump without = value
        let field: syn::Field = parse_quote! {
            #[light_account(init, associated_token, owner, mint, bump)]
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
        // F006: Duplicate keys should be rejected
        let field: syn::Field = parse_quote! {
            #[light_account(token, authority = [b"auth1"], authority = [b"auth2"])]
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
        // F006: Duplicate keys in associated_token should also be rejected
        let field: syn::Field = parse_quote! {
            #[light_account(init, associated_token, owner = foo, owner = bar, mint)]
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
        // F007: Empty authority seeds with init should be rejected
        let field: syn::Field = parse_quote! {
            #[light_account(init, token, authority = [])]
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
        // F007: Empty authority seeds without init should be allowed (mark-only mode)
        let field: syn::Field = parse_quote! {
            #[light_account(token, authority = [])]
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
        assert!(result.is_ok(), "Should parse successfully with direct proof arg");
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
                mint_signer = mint_signer,
                authority = authority,
                decimals = 9,
                mint_seeds = &[b"test"]
            )]
            pub cmint: UncheckedAccount<'info>
        };
        let field_ident = field.ident.clone().unwrap();

        // Simulate passing CreateAccountsProof as direct arg named "create_proof"
        let proof_ident: Ident = parse_quote!(create_proof);
        let direct_proof_arg = Some(proof_ident.clone());

        let result = parse_light_account_attr(&field, &field_ident, &direct_proof_arg);
        assert!(result.is_ok(), "Should parse successfully with direct proof arg");
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
            }
            _ => panic!("Expected Mint field"),
        }
    }
}
