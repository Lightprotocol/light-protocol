//! Unified Accounts struct parsing for Light Protocol macros.
//!
//! This module provides `ParsedAccountsStruct`, the unified parsed representation
//! of an Anchor Accounts struct for `#[derive(LightAccounts)]`.

use syn::{DeriveInput, Error, Expr, Ident, ItemStruct, Type};

use super::{
    infra::{InfraFieldClassifier, InfraFields},
    instruction_arg::{args_to_set, parse_instruction_attr, InstructionArg, InstructionArgSet},
};
use crate::light_pdas::seeds::ClassifiedSeed;

// Type aliases for field types from accounts module
type ParsedAtaField = crate::light_pdas::accounts::light_account::AtaField;
type ParsedTokenField = crate::light_pdas::accounts::light_account::TokenAccountField;
type ParsedMintField = crate::light_pdas::accounts::mint::LightMintField;

// ============================================================================
// Unified Parsed Types
// ============================================================================

/// Unified parsed representation of an Anchor Accounts struct.
#[derive(Debug)]
pub struct ParsedAccountsStruct {
    /// Struct identifier
    pub struct_name: Ident,
    /// Generics from the struct definition
    pub generics: syn::Generics,
    /// Fields marked with `#[light_account(init)]` for compressed PDAs
    pub pda_fields: Vec<ParsedPdaField>,
    /// Fields marked with `#[light_account(init, mint::...)]` for compressed mints
    pub mint_fields: Vec<ParsedMintField>,
    /// Fields marked with `#[light_account([init,] token::...)]` for token accounts
    pub token_fields: Vec<ParsedTokenField>,
    /// Fields marked with `#[light_account([init,] associated_token::...)]` for ATAs
    pub ata_fields: Vec<ParsedAtaField>,
    /// Parsed instruction arguments from `#[instruction(...)]`
    pub instruction_args: Option<Vec<InstructionArg>>,
    /// Infrastructure fields detected by naming convention
    pub infra_fields: InfraFields,
    /// If CreateAccountsProof is passed as a direct instruction arg, stores arg name
    pub direct_proof_arg: Option<Ident>,
}

/// A field marked with `#[light_account(init)]` for compressed PDA.
#[derive(Debug, Clone)]
pub struct ParsedPdaField {
    /// Field name in the struct (e.g., `user_record`)
    pub field_name: Ident,
    /// True if the field is `Box<Account<T>>`
    pub is_boxed: bool,
    /// True if the field uses zero-copy serialization (`AccountLoader`)
    pub is_zero_copy: bool,
    /// Address tree info expression (for code generation)
    pub address_tree_info: Option<Expr>,
    /// Output tree index expression (for code generation)
    pub output_tree: Option<Expr>,
}

// ============================================================================
// Parsing Functions
// ============================================================================

/// Parse an Accounts struct for derive macro.
fn parse_accounts_struct_impl(
    input: &ItemStruct,
    direct_proof_arg: Option<Ident>,
) -> Result<ParsedAccountsStruct, Error> {
    let struct_name = input.ident.clone();
    let generics = input.generics.clone();

    // Parse instruction args
    let instruction_args = parse_instruction_attr(&input.attrs)?;
    let instruction_arg_set = match &instruction_args {
        Some(args) => args_to_set(args),
        None => InstructionArgSet::empty(),
    };

    // Get fields
    let fields = match &input.fields {
        syn::Fields::Named(fields) => &fields.named,
        _ => {
            return Err(Error::new_spanned(
                input,
                "expected struct with named fields",
            ));
        }
    };

    let mut pda_fields = Vec::new();
    let mut mint_fields = Vec::new();
    let mut token_fields = Vec::new();
    let mut ata_fields = Vec::new();
    let mut infra_fields = InfraFields::default();

    for field in fields {
        let field_ident = field
            .ident
            .clone()
            .ok_or_else(|| Error::new_spanned(field, "expected named field with identifier"))?;
        let field_name = field_ident.to_string();

        // Track infrastructure fields by naming convention
        if let Some(field_type) = InfraFieldClassifier::classify(&field_name) {
            infra_fields.set(field_type, field_ident.clone())?;
        }

        // Check for #[light_account(...)] attribute
        if let Some(light_account_field) =
            crate::light_pdas::accounts::light_account::parse_light_account_attr(
                field,
                &field_ident,
                &direct_proof_arg,
            )?
        {
            use crate::light_pdas::accounts::light_account::LightAccountField;

            match light_account_field {
                LightAccountField::Pda(pda) => {
                    // Extract seeds for validation (not stored, just validated)
                    let _seeds: Vec<ClassifiedSeed> =
                        crate::light_pdas::seeds::anchor_extraction::extract_anchor_seeds(
                            &field.attrs,
                            &instruction_arg_set,
                        )?;

                    pda_fields.push(ParsedPdaField {
                        field_name: field_ident,
                        is_boxed: pda.is_boxed,
                        is_zero_copy: pda.is_zero_copy,
                        address_tree_info: Some(pda.address_tree_info),
                        output_tree: Some(pda.output_tree),
                    });
                }
                LightAccountField::Mint(mint) => {
                    mint_fields.push(*mint);
                }
                LightAccountField::TokenAccount(token) => {
                    token_fields.push(*token);
                }
                LightAccountField::AssociatedToken(ata) => {
                    ata_fields.push(*ata);
                }
            }
        }
    }

    // Validation: #[light_account] fields require #[instruction] attribute
    let has_light_account_fields = !pda_fields.is_empty()
        || !mint_fields.is_empty()
        || !token_fields.is_empty()
        || !ata_fields.is_empty();

    if has_light_account_fields && instruction_args.is_none() {
        return Err(Error::new_spanned(
            input,
            "#[light_account] fields require #[instruction(params: YourParamsType)] \
             attribute on the struct",
        ));
    }

    Ok(ParsedAccountsStruct {
        struct_name,
        generics,
        pda_fields,
        mint_fields,
        token_fields,
        ata_fields,
        instruction_args,
        infra_fields,
        direct_proof_arg,
    })
}

/// Parse a DeriveInput (from derive macro) into ParsedAccountsStruct.
///
/// This is the main entry point for the `#[derive(LightAccounts)]` macro.
pub fn parse_derive_input(input: &DeriveInput) -> Result<ParsedAccountsStruct, Error> {
    // First parse instruction args to find direct proof arg
    let instruction_args = parse_instruction_attr(&input.attrs)?;
    let direct_proof_arg = find_direct_proof_arg(&instruction_args)?;

    // Convert DeriveInput to ItemStruct-like parsing
    match &input.data {
        syn::Data::Struct(data) => {
            // Create a temporary ItemStruct for parsing
            let item_struct = ItemStruct {
                attrs: input.attrs.clone(),
                vis: input.vis.clone(),
                struct_token: data.struct_token,
                ident: input.ident.clone(),
                generics: input.generics.clone(),
                fields: data.fields.clone(),
                semi_token: data.semi_token,
            };

            parse_accounts_struct_impl(&item_struct, direct_proof_arg)
        }
        _ => Err(Error::new_spanned(input, "expected struct")),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if a type is `CreateAccountsProof` (match last path segment).
fn is_create_accounts_proof_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "CreateAccountsProof";
        }
    }
    false
}

/// Find if any instruction argument has type `CreateAccountsProof`.
fn find_direct_proof_arg(
    instruction_args: &Option<Vec<InstructionArg>>,
) -> Result<Option<Ident>, Error> {
    let Some(args) = instruction_args.as_ref() else {
        return Ok(None);
    };

    let proof_args: Vec<_> = args
        .iter()
        .filter(|arg| is_create_accounts_proof_type(&arg.ty))
        .collect();

    match proof_args.len() {
        0 => Ok(None),
        1 => Ok(Some(proof_args[0].name.clone())),
        _ => {
            let names: Vec<_> = proof_args.iter().map(|a| a.name.to_string()).collect();
            Err(Error::new_spanned(
                &proof_args[1].name,
                format!(
                    "Multiple CreateAccountsProof arguments found: [{}]. \
                     Only one CreateAccountsProof argument is allowed per instruction.",
                    names.join(", ")
                ),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_parse_empty_struct() {
        let input: DeriveInput = parse_quote! {
            #[derive(Accounts)]
            pub struct Empty<'info> {
                pub fee_payer: Signer<'info>,
            }
        };

        let result = parse_derive_input(&input);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed.pda_fields.is_empty());
    }

    #[test]
    fn test_parse_with_pda_field() {
        let input: DeriveInput = parse_quote! {
            #[derive(Accounts)]
            #[instruction(params: CreateParams)]
            pub struct Create<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,

                #[account(init, payer = fee_payer, space = 100, seeds = [b"user"], bump)]
                #[light_account(init)]
                pub user_record: Account<'info, UserRecord>,
            }
        };

        let result = parse_derive_input(&input);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.pda_fields.len(), 1);
        assert_eq!(parsed.pda_fields[0].field_name.to_string(), "user_record");
    }

    #[test]
    fn test_parse_infra_fields() {
        let input: DeriveInput = parse_quote! {
            #[derive(Accounts)]
            pub struct Test<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,

                pub compression_config: AccountInfo<'info>,
            }
        };

        let result = parse_derive_input(&input);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed.infra_fields.fee_payer.is_some());
        assert!(parsed.infra_fields.compression_config.is_some());
    }

    #[test]
    fn test_light_account_without_instruction_fails() {
        let input: DeriveInput = parse_quote! {
            #[derive(Accounts)]
            pub struct NoInstruction<'info> {
                #[account(init, payer = fee_payer, space = 100, seeds = [b"user"], bump)]
                #[light_account(init)]
                pub user_record: Account<'info, UserRecord>,
            }
        };

        let result = parse_derive_input(&input);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("#[instruction"));
    }
}
