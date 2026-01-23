//! Property-based tests for accounts parsing and classification.
//!
//! These tests verify correctness properties of:
//! - `InfraFieldClassifier::classify` - Infrastructure field classification
//! - `InfraFields::set` - Infrastructure field state management

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use syn::Ident;

    // Access parse module from parent (accounts module)
    use crate::light_pdas::accounts::parse::{InfraFieldClassifier, InfraFieldType, InfraFields};

    // ========================================================================
    // Helper functions
    // ========================================================================

    /// Creates an Ident from a string (for testing purposes)
    fn make_ident(name: &str) -> Ident {
        syn::parse_str::<Ident>(name)
            .unwrap_or_else(|_| syn::parse_str::<Ident>("test_ident").unwrap())
    }

    /// All InfraFieldType variants
    fn all_infra_types() -> Vec<InfraFieldType> {
        vec![
            InfraFieldType::FeePayer,
            InfraFieldType::CompressionConfig,
            InfraFieldType::LightTokenConfig,
            InfraFieldType::LightTokenRentSponsor,
            InfraFieldType::LightTokenProgram,
            InfraFieldType::LightTokenCpiAuthority,
        ]
    }

    /// All known field names that map to InfraFieldType
    fn all_known_field_names() -> Vec<&'static str> {
        vec![
            "fee_payer",
            "payer",
            "creator",
            "compression_config",
            "light_token_compressible_config",
            "light_token_rent_sponsor",
            "rent_sponsor",
            "light_token_program",
            "light_token_cpi_authority",
        ]
    }

    // ========================================================================
    // Strategies for generating test inputs
    // ========================================================================

    /// Strategy for generating known field names
    fn arb_known_field_name() -> impl Strategy<Value = &'static str> {
        prop::sample::select(all_known_field_names())
    }

    /// Strategy for generating random lowercase identifiers (likely unknown)
    fn arb_random_field_name() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{2,20}"
    }

    /// Strategy for generating a random InfraFieldType
    fn arb_infra_field_type() -> impl Strategy<Value = InfraFieldType> {
        prop::sample::select(all_infra_types())
    }

    // ========================================================================
    // Property Tests: InfraFieldClassifier::classify
    // ========================================================================

    proptest! {
        /// Known field names should be classified correctly.
        #[test]
        fn prop_known_names_classified(name in arb_known_field_name()) {
            let result = InfraFieldClassifier::classify(name);
            prop_assert!(
                result.is_some(),
                "Known field name '{}' should be classified",
                name
            );
        }

        /// All accepted names for each type should classify to that type.
        #[test]
        fn prop_all_accepted_names_work(_seed in 0u32..1000) {
            for field_type in all_infra_types() {
                for name in field_type.accepted_names() {
                    let result = InfraFieldClassifier::classify(name);
                    prop_assert!(
                        result == Some(field_type),
                        "Name '{}' should classify to {:?}, got {:?}",
                        name, field_type, result
                    );
                }
            }
        }

        /// Unknown field names should return None.
        #[test]
        fn prop_unknown_names_return_none(name in arb_random_field_name()) {
            // Skip if randomly generated a known name
            prop_assume!(!all_known_field_names().contains(&name.as_str()));

            let result = InfraFieldClassifier::classify(&name);
            prop_assert!(
                result.is_none(),
                "Unknown field name '{}' should return None, got {:?}",
                name, result
            );
        }

        /// Classification should be deterministic.
        #[test]
        fn prop_classify_deterministic(name in arb_random_field_name()) {
            let result1 = InfraFieldClassifier::classify(&name);
            let result2 = InfraFieldClassifier::classify(&name);
            prop_assert_eq!(
                result1, result2,
                "Classification should be deterministic for '{}'",
                name
            );
        }

        /// Each accepted name should map to exactly one type (bijection check).
        #[test]
        fn prop_bijection(_seed in 0u32..1000) {
            let infra_types = all_infra_types();
            for name in all_known_field_names() {
                let result = InfraFieldClassifier::classify(name);
                // Count how many types accept this name
                let matching_count = infra_types
                    .iter()
                    .filter(|t| t.accepted_names().contains(&name))
                    .count();

                prop_assert_eq!(
                    matching_count, 1,
                    "Name '{}' should map to exactly one type",
                    name
                );
                prop_assert!(
                    result.is_some(),
                    "Known name '{}' should classify successfully",
                    name
                );
            }
        }

        /// All 6 InfraFieldType variants should be reachable via classification.
        #[test]
        fn prop_exhaustive_coverage(_seed in 0u32..1000) {
            let mut covered = vec![false; 6];

            for name in all_known_field_names() {
                if let Some(field_type) = InfraFieldClassifier::classify(name) {
                    let index = match field_type {
                        InfraFieldType::FeePayer => 0,
                        InfraFieldType::CompressionConfig => 1,
                        InfraFieldType::LightTokenConfig => 2,
                        InfraFieldType::LightTokenRentSponsor => 3,
                        InfraFieldType::LightTokenProgram => 4,
                        InfraFieldType::LightTokenCpiAuthority => 5,
                    };
                    covered[index] = true;
                }
            }

            prop_assert!(
                covered.iter().all(|&c| c),
                "Not all InfraFieldType variants are reachable: {:?}",
                covered
            );
        }
    }

    // ========================================================================
    // Property Tests: InfraFields::set
    // ========================================================================

    proptest! {
        /// First insert of any field type should succeed.
        #[test]
        fn prop_first_insert_succeeds(field_type in arb_infra_field_type()) {
            let mut fields = InfraFields::default();
            let ident = make_ident("test_field");

            let result = fields.set(field_type, ident);
            prop_assert!(
                result.is_ok(),
                "First insert of {:?} should succeed",
                field_type
            );
        }

        /// Duplicate insert of same field type should fail.
        #[test]
        fn prop_duplicate_insert_fails(field_type in arb_infra_field_type()) {
            let mut fields = InfraFields::default();
            let ident1 = make_ident("first_field");
            let ident2 = make_ident("second_field");

            // First insert should succeed
            let result1 = fields.set(field_type, ident1);
            prop_assert!(result1.is_ok(), "First insert should succeed");

            // Second insert of same type should fail
            let result2 = fields.set(field_type, ident2);
            prop_assert!(
                result2.is_err(),
                "Duplicate insert of {:?} should fail",
                field_type
            );
        }

        /// Different field types can coexist.
        #[test]
        fn prop_different_types_coexist(_seed in 0u32..1000) {
            let mut fields = InfraFields::default();

            for (i, field_type) in all_infra_types().into_iter().enumerate() {
                let ident = make_ident(&format!("field_{}", i));
                let result = fields.set(field_type, ident);
                prop_assert!(
                    result.is_ok(),
                    "Insert of different type {:?} should succeed",
                    field_type
                );
            }
        }

        /// After set, corresponding Option field should be Some.
        #[test]
        fn prop_state_mutation_correct(field_type in arb_infra_field_type()) {
            let mut fields = InfraFields::default();
            let ident = make_ident("test_field");

            fields.set(field_type, ident).unwrap();

            let is_set = match field_type {
                InfraFieldType::FeePayer => fields.fee_payer.is_some(),
                InfraFieldType::CompressionConfig => fields.compression_config.is_some(),
                InfraFieldType::LightTokenConfig => fields.light_token_config.is_some(),
                InfraFieldType::LightTokenRentSponsor => fields.light_token_rent_sponsor.is_some(),
                InfraFieldType::LightTokenProgram => fields.light_token_program.is_some(),
                InfraFieldType::LightTokenCpiAuthority => fields.light_token_cpi_authority.is_some(),
            };

            prop_assert!(
                is_set,
                "After set({:?}), corresponding field should be Some",
                field_type
            );
        }

        /// Error message should identify the duplicate field type.
        #[test]
        fn prop_error_identifies_field(field_type in arb_infra_field_type()) {
            let mut fields = InfraFields::default();
            let ident1 = make_ident("first");
            let ident2 = make_ident("second");

            fields.set(field_type, ident1).unwrap();
            let result = fields.set(field_type, ident2);

            if let Err(err) = result {
                let err_msg = err.to_string();
                prop_assert!(
                    err_msg.contains("duplicate"),
                    "Error message should mention 'duplicate', got: {}",
                    err_msg
                );
            }
        }

        /// Other fields should remain None after setting one field.
        #[test]
        fn prop_other_fields_unchanged(field_type in arb_infra_field_type()) {
            let mut fields = InfraFields::default();
            let ident = make_ident("test_field");

            fields.set(field_type, ident).unwrap();

            // Count how many fields are set
            let set_count = [
                fields.fee_payer.is_some(),
                fields.compression_config.is_some(),
                fields.light_token_config.is_some(),
                fields.light_token_rent_sponsor.is_some(),
                fields.light_token_program.is_some(),
                fields.light_token_cpi_authority.is_some(),
            ].iter().filter(|&&x| x).count();

            prop_assert_eq!(
                set_count, 1,
                "Only one field should be set after single set() call for {:?}",
                field_type
            );
        }
    }

    // ========================================================================
    // Property Tests: InfraFieldType methods
    // ========================================================================

    proptest! {
        /// Each InfraFieldType should have at least one accepted name.
        #[test]
        fn prop_each_type_has_accepted_names(field_type in arb_infra_field_type()) {
            let names = field_type.accepted_names();
            prop_assert!(
                !names.is_empty(),
                "{:?} should have at least one accepted name",
                field_type
            );
        }

        /// Each InfraFieldType should have a non-empty description.
        #[test]
        fn prop_each_type_has_description(field_type in arb_infra_field_type()) {
            let desc = field_type.description();
            prop_assert!(
                !desc.is_empty(),
                "{:?} should have a non-empty description",
                field_type
            );
        }

        /// accepted_names should be deterministic.
        #[test]
        fn prop_accepted_names_deterministic(field_type in arb_infra_field_type()) {
            let names1 = field_type.accepted_names();
            let names2 = field_type.accepted_names();
            prop_assert_eq!(
                names1, names2,
                "accepted_names should be deterministic for {:?}",
                field_type
            );
        }
    }
}
