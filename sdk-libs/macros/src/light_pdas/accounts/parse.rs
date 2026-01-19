//! Parsing logic for #[light_account(...)] attributes.
//!
//! This module handles struct-level parsing and field classification.
//! The unified #[light_account] attribute parsing is in `light_account.rs`.

use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    DeriveInput, Error, Expr, Ident, Token, Type,
};

// Import unified parsing from light_account module
use super::light_account::{parse_light_account_attr, LightAccountField};
// Import LightMintField from mint module (for type export)
pub(super) use super::mint::LightMintField;

// ============================================================================
// Infrastructure Field Classification
// ============================================================================

/// Classification of infrastructure fields by naming convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InfraFieldType {
    FeePayer,
    CompressionConfig,
    LightTokenConfig,
    LightTokenRentSponsor,
    LightTokenProgram,
    LightTokenCpiAuthority,
}

impl InfraFieldType {
    /// Returns the accepted field names for this infrastructure type.
    pub fn accepted_names(&self) -> &'static [&'static str] {
        match self {
            InfraFieldType::FeePayer => &["fee_payer", "payer", "creator"],
            InfraFieldType::CompressionConfig => &["compression_config"],
            InfraFieldType::LightTokenConfig => &["light_token_compressible_config"],
            InfraFieldType::LightTokenRentSponsor => &["light_token_rent_sponsor", "rent_sponsor"],
            InfraFieldType::LightTokenProgram => &["light_token_program"],
            InfraFieldType::LightTokenCpiAuthority => &["light_token_cpi_authority"],
        }
    }

    /// Human-readable description for error messages.
    pub fn description(&self) -> &'static str {
        match self {
            InfraFieldType::FeePayer => "fee payer (transaction signer)",
            InfraFieldType::CompressionConfig => "compression config",
            InfraFieldType::LightTokenConfig => "light token compressible config",
            InfraFieldType::LightTokenRentSponsor => "light token rent sponsor",
            InfraFieldType::LightTokenProgram => "light token program",
            InfraFieldType::LightTokenCpiAuthority => "light token CPI authority",
        }
    }
}

/// Classifier for infrastructure fields by naming convention.
pub(super) struct InfraFieldClassifier;

impl InfraFieldClassifier {
    /// Classify a field name into its infrastructure type, if any.
    #[inline]
    pub fn classify(name: &str) -> Option<InfraFieldType> {
        match name {
            "fee_payer" | "payer" | "creator" => Some(InfraFieldType::FeePayer),
            "compression_config" => Some(InfraFieldType::CompressionConfig),
            "light_token_compressible_config" => Some(InfraFieldType::LightTokenConfig),
            "light_token_rent_sponsor" | "rent_sponsor" => {
                Some(InfraFieldType::LightTokenRentSponsor)
            }
            "light_token_program" => Some(InfraFieldType::LightTokenProgram),
            "light_token_cpi_authority" => Some(InfraFieldType::LightTokenCpiAuthority),
            _ => None,
        }
    }
}

/// Collected infrastructure field identifiers.
#[derive(Default)]
pub(super) struct InfraFields {
    pub fee_payer: Option<Ident>,
    pub compression_config: Option<Ident>,
    pub light_token_config: Option<Ident>,
    pub light_token_rent_sponsor: Option<Ident>,
    pub light_token_program: Option<Ident>,
    pub light_token_cpi_authority: Option<Ident>,
}

impl InfraFields {
    /// Set an infrastructure field by type.
    pub fn set(&mut self, field_type: InfraFieldType, ident: Ident) {
        match field_type {
            InfraFieldType::FeePayer => self.fee_payer = Some(ident),
            InfraFieldType::CompressionConfig => self.compression_config = Some(ident),
            InfraFieldType::LightTokenConfig => self.light_token_config = Some(ident),
            InfraFieldType::LightTokenRentSponsor => self.light_token_rent_sponsor = Some(ident),
            InfraFieldType::LightTokenProgram => self.light_token_program = Some(ident),
            InfraFieldType::LightTokenCpiAuthority => self.light_token_cpi_authority = Some(ident),
        }
    }
}

/// Parsed representation of a struct with rentfree and light_mint fields.
pub(super) struct ParsedLightAccountsStruct {
    pub struct_name: Ident,
    pub generics: syn::Generics,
    pub rentfree_fields: Vec<ParsedPdaField>,
    pub light_mint_fields: Vec<LightMintField>,
    pub instruction_args: Option<Vec<InstructionArg>>,
    /// Infrastructure fields detected by naming convention.
    pub infra_fields: InfraFields,
}

/// A field marked with #[light_account(init)]
pub(super) struct ParsedPdaField {
    pub ident: Ident,
    /// The inner type T from Account<'info, T> or Box<Account<'info, T>>
    /// Preserves the full type path (e.g., crate::state::UserRecord).
    pub inner_type: Type,
    pub address_tree_info: Expr,
    pub output_tree: Expr,
    /// True if the field is Box<Account<T>>, false if Account<T>
    pub is_boxed: bool,
}

/// Instruction argument from #[instruction(...)]
pub(super) struct InstructionArg {
    pub name: Ident,
    pub ty: Type,
}

impl Parse for InstructionArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        Ok(Self { name, ty })
    }
}

/// Parse #[instruction(...)] attribute from struct.
///
/// Returns `Ok(None)` if no instruction attribute is present,
/// `Ok(Some(args))` if successfully parsed, or `Err` on malformed syntax.
fn parse_instruction_attr(attrs: &[syn::Attribute]) -> Result<Option<Vec<InstructionArg>>, Error> {
    for attr in attrs {
        if attr.path().is_ident("instruction") {
            let args = attr.parse_args_with(|input: ParseStream| {
                let content: Punctuated<InstructionArg, Token![,]> =
                    Punctuated::parse_terminated(input)?;
                Ok(content.into_iter().collect::<Vec<_>>())
            })?;
            return Ok(Some(args));
        }
    }
    Ok(None)
}

/// Parse a struct to extract light_account fields (PDAs and mints).
pub(super) fn parse_light_accounts_struct(
    input: &DeriveInput,
) -> Result<ParsedLightAccountsStruct, Error> {
    let struct_name = input.ident.clone();
    let generics = input.generics.clone();

    let instruction_args = parse_instruction_attr(&input.attrs)?;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => return Err(Error::new_spanned(input, "expected named fields")),
        },
        _ => return Err(Error::new_spanned(input, "expected struct")),
    };

    let mut rentfree_fields = Vec::new();
    let mut light_mint_fields = Vec::new();
    let mut infra_fields = InfraFields::default();

    for field in fields {
        let field_ident = field
            .ident
            .clone()
            .ok_or_else(|| Error::new_spanned(field, "expected named field with identifier"))?;
        let field_name = field_ident.to_string();

        // Track infrastructure fields by naming convention using the classifier.
        // See InfraFieldClassifier for supported field names.
        if let Some(field_type) = InfraFieldClassifier::classify(&field_name) {
            infra_fields.set(field_type, field_ident.clone());
        }

        // Check for #[light_account(...)] - the unified syntax
        if let Some(light_account_field) = parse_light_account_attr(field, &field_ident)? {
            match light_account_field {
                LightAccountField::Pda(pda) => rentfree_fields.push((*pda).into()),
                LightAccountField::Mint(mint) => light_mint_fields.push(*mint),
            }
            continue; // Field processed, move to next
        }

        // Check for deprecated syntax and provide helpful error messages
        for attr in &field.attrs {
            if attr.path().is_ident("rentfree") {
                return Err(Error::new_spanned(
                    attr,
                    "The #[light_account(init)] attribute has been replaced. \
                     Use #[light_account(init)] for PDAs or \
                     #[light_account(init, address_tree_info = ..., output_tree = ...)] with options.",
                ));
            }
            if attr.path().is_ident("light_mint") {
                return Err(Error::new_spanned(
                    attr,
                    "The #[light_account(init, mint,...)] attribute has been replaced. \
                     Use #[light_account(init, mint, mint_signer = ..., authority = ..., decimals = ..., mint_seeds = ...)].",
                ));
            }

            // TODO(future): Add parsing for #[light_account(token, ...)] attribute for token accounts and ATAs.
            // Would need LightTokenField struct with: field_ident, authority_seeds, mint field ref.
        }
    }

    // Validation: #[light_account] fields require #[instruction] attribute
    if (!rentfree_fields.is_empty() || !light_mint_fields.is_empty()) && instruction_args.is_none()
    {
        return Err(Error::new_spanned(
            input,
            "#[light_account] fields require #[instruction(params: YourParamsType)] \
             attribute on the struct",
        ));
    }

    Ok(ParsedLightAccountsStruct {
        struct_name,
        generics,
        rentfree_fields,
        light_mint_fields,
        instruction_args,
        infra_fields,
    })
}
