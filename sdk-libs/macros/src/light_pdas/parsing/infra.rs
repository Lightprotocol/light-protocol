//! Infrastructure field classification for Light Protocol Accounts structs.
//!
//! This module provides classification of infrastructure fields by naming convention,
//! detecting fields like `fee_payer`, `compression_config`, `light_token_program`, etc.

use syn::{Error, Ident};

// ============================================================================
// Infrastructure Field Classification
// ============================================================================

/// Classification of infrastructure fields by naming convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfraFieldType {
    FeePayer,
    CompressionConfig,
    PdaRentSponsor,
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
            InfraFieldType::PdaRentSponsor => &["pda_rent_sponsor", "compression_rent_sponsor"],
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
            InfraFieldType::PdaRentSponsor => "PDA rent sponsor (for rent reimbursement)",
            InfraFieldType::LightTokenConfig => "light token compressible config",
            InfraFieldType::LightTokenRentSponsor => "light token rent sponsor",
            InfraFieldType::LightTokenProgram => "light token program",
            InfraFieldType::LightTokenCpiAuthority => "light token CPI authority",
        }
    }
}

/// Classifier for infrastructure fields by naming convention.
pub struct InfraFieldClassifier;

impl InfraFieldClassifier {
    /// Classify a field name into its infrastructure type, if any.
    #[inline]
    pub fn classify(name: &str) -> Option<InfraFieldType> {
        match name {
            "fee_payer" | "payer" | "creator" => Some(InfraFieldType::FeePayer),
            "compression_config" => Some(InfraFieldType::CompressionConfig),
            "pda_rent_sponsor" | "compression_rent_sponsor" => Some(InfraFieldType::PdaRentSponsor),
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
#[derive(Default, Debug)]
pub struct InfraFields {
    pub fee_payer: Option<Ident>,
    pub compression_config: Option<Ident>,
    pub pda_rent_sponsor: Option<Ident>,
    pub light_token_config: Option<Ident>,
    pub light_token_rent_sponsor: Option<Ident>,
    pub light_token_program: Option<Ident>,
    pub light_token_cpi_authority: Option<Ident>,
}

impl InfraFields {
    /// Set an infrastructure field by type.
    /// Returns an error if the field is already set (duplicate detection).
    pub fn set(&mut self, field_type: InfraFieldType, ident: Ident) -> Result<(), Error> {
        match field_type {
            InfraFieldType::FeePayer => {
                if let Some(ref existing) = self.fee_payer {
                    return Err(Error::new_spanned(
                        &ident,
                        format!(
                            "Duplicate fee payer: `{}` conflicts with `{}`. Only one of {} allowed.",
                            ident,
                            existing,
                            InfraFieldType::FeePayer.accepted_names().join(", ")
                        ),
                    ));
                }
                self.fee_payer = Some(ident);
            }
            InfraFieldType::CompressionConfig => {
                if let Some(ref existing) = self.compression_config {
                    return Err(Error::new_spanned(
                        &ident,
                        format!(
                            "Duplicate compression config: `{}` conflicts with `{}`. Only one of {} allowed.",
                            ident,
                            existing,
                            InfraFieldType::CompressionConfig.accepted_names().join(", ")
                        ),
                    ));
                }
                self.compression_config = Some(ident);
            }
            InfraFieldType::PdaRentSponsor => {
                if let Some(ref existing) = self.pda_rent_sponsor {
                    return Err(Error::new_spanned(
                        &ident,
                        format!(
                            "Duplicate PDA rent sponsor: `{}` conflicts with `{}`. Only one of {} allowed.",
                            ident,
                            existing,
                            InfraFieldType::PdaRentSponsor.accepted_names().join(", ")
                        ),
                    ));
                }
                self.pda_rent_sponsor = Some(ident);
            }
            InfraFieldType::LightTokenConfig => {
                if let Some(ref existing) = self.light_token_config {
                    return Err(Error::new_spanned(
                        &ident,
                        format!(
                            "Duplicate light token config: `{}` conflicts with `{}`. Only one of {} allowed.",
                            ident,
                            existing,
                            InfraFieldType::LightTokenConfig.accepted_names().join(", ")
                        ),
                    ));
                }
                self.light_token_config = Some(ident);
            }
            InfraFieldType::LightTokenRentSponsor => {
                if let Some(ref existing) = self.light_token_rent_sponsor {
                    return Err(Error::new_spanned(
                        &ident,
                        format!(
                            "Duplicate light token rent sponsor: `{}` conflicts with `{}`. Only one of {} allowed.",
                            ident,
                            existing,
                            InfraFieldType::LightTokenRentSponsor.accepted_names().join(", ")
                        ),
                    ));
                }
                self.light_token_rent_sponsor = Some(ident);
            }
            InfraFieldType::LightTokenProgram => {
                if let Some(ref existing) = self.light_token_program {
                    return Err(Error::new_spanned(
                        &ident,
                        format!(
                            "Duplicate light token program: `{}` conflicts with `{}`. Only one of {} allowed.",
                            ident,
                            existing,
                            InfraFieldType::LightTokenProgram.accepted_names().join(", ")
                        ),
                    ));
                }
                self.light_token_program = Some(ident);
            }
            InfraFieldType::LightTokenCpiAuthority => {
                if let Some(ref existing) = self.light_token_cpi_authority {
                    return Err(Error::new_spanned(
                        &ident,
                        format!(
                            "Duplicate light token CPI authority: `{}` conflicts with `{}`. Only one of {} allowed.",
                            ident,
                            existing,
                            InfraFieldType::LightTokenCpiAuthority.accepted_names().join(", ")
                        ),
                    ));
                }
                self.light_token_cpi_authority = Some(ident);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_fee_payer() {
        assert_eq!(
            InfraFieldClassifier::classify("fee_payer"),
            Some(InfraFieldType::FeePayer)
        );
        assert_eq!(
            InfraFieldClassifier::classify("payer"),
            Some(InfraFieldType::FeePayer)
        );
        assert_eq!(
            InfraFieldClassifier::classify("creator"),
            Some(InfraFieldType::FeePayer)
        );
    }

    #[test]
    fn test_classify_compression_config() {
        assert_eq!(
            InfraFieldClassifier::classify("compression_config"),
            Some(InfraFieldType::CompressionConfig)
        );
    }

    #[test]
    fn test_classify_rent_sponsor() {
        assert_eq!(
            InfraFieldClassifier::classify("pda_rent_sponsor"),
            Some(InfraFieldType::PdaRentSponsor)
        );
        assert_eq!(
            InfraFieldClassifier::classify("compression_rent_sponsor"),
            Some(InfraFieldType::PdaRentSponsor)
        );
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(InfraFieldClassifier::classify("unknown_field"), None);
        assert_eq!(InfraFieldClassifier::classify("authority"), None);
    }

    #[test]
    fn test_infra_fields_set_duplicate() {
        use syn::parse_quote;

        let mut fields = InfraFields::default();
        let ident1: Ident = parse_quote!(fee_payer);
        let ident2: Ident = parse_quote!(payer);

        assert!(fields.set(InfraFieldType::FeePayer, ident1).is_ok());
        assert!(fields.set(InfraFieldType::FeePayer, ident2).is_err());
    }
}
