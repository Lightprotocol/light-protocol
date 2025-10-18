use light_hasher::HasherError;
use light_sdk::error::LightSdkError as SolanaLightSdkError;
use light_sdk_pinocchio::error::LightSdkError as PinocchioLightSdkError;

fn generate_all_solana_errors() -> Vec<SolanaLightSdkError> {
    vec![
        SolanaLightSdkError::ConstraintViolation,
        SolanaLightSdkError::InvalidLightSystemProgram,
        SolanaLightSdkError::ExpectedAccounts,
        SolanaLightSdkError::ExpectedAddressTreeInfo,
        SolanaLightSdkError::ExpectedAddressRootIndex,
        SolanaLightSdkError::ExpectedData,
        SolanaLightSdkError::ExpectedDiscriminator,
        SolanaLightSdkError::ExpectedHash,
        SolanaLightSdkError::ExpectedLightSystemAccount("test".to_string()),
        SolanaLightSdkError::ExpectedMerkleContext,
        SolanaLightSdkError::ExpectedRootIndex,
        SolanaLightSdkError::TransferFromNoInput,
        SolanaLightSdkError::TransferFromNoLamports,
        SolanaLightSdkError::TransferFromInsufficientLamports,
        SolanaLightSdkError::TransferIntegerOverflow,
        SolanaLightSdkError::Borsh,
        SolanaLightSdkError::FewerAccountsThanSystemAccounts,
        SolanaLightSdkError::InvalidCpiSignerAccount,
        SolanaLightSdkError::MissingField("test".to_string()),
        SolanaLightSdkError::OutputStateTreeIndexIsNone,
        SolanaLightSdkError::InitAddressIsNone,
        SolanaLightSdkError::InitWithAddressIsNone,
        SolanaLightSdkError::InitWithAddressOutputIsNone,
        SolanaLightSdkError::MetaMutAddressIsNone,
        SolanaLightSdkError::MetaMutInputIsNone,
        SolanaLightSdkError::MetaMutOutputLamportsIsNone,
        SolanaLightSdkError::MetaMutOutputIsNone,
        SolanaLightSdkError::MetaCloseAddressIsNone,
        SolanaLightSdkError::MetaCloseInputIsNone,
        SolanaLightSdkError::CpiAccountsIndexOutOfBounds(1),
        SolanaLightSdkError::Hasher(HasherError::IntegerOverflow),
    ]
}

fn generate_all_pinocchio_errors() -> Vec<PinocchioLightSdkError> {
    vec![
        PinocchioLightSdkError::ConstraintViolation,
        PinocchioLightSdkError::InvalidLightSystemProgram,
        PinocchioLightSdkError::ExpectedAccounts,
        PinocchioLightSdkError::ExpectedAddressTreeInfo,
        PinocchioLightSdkError::ExpectedAddressRootIndex,
        PinocchioLightSdkError::ExpectedData,
        PinocchioLightSdkError::ExpectedDiscriminator,
        PinocchioLightSdkError::ExpectedHash,
        PinocchioLightSdkError::ExpectedLightSystemAccount("test".to_string()),
        PinocchioLightSdkError::ExpectedMerkleContext,
        PinocchioLightSdkError::ExpectedRootIndex,
        PinocchioLightSdkError::TransferFromNoInput,
        PinocchioLightSdkError::TransferFromNoLamports,
        PinocchioLightSdkError::TransferFromInsufficientLamports,
        PinocchioLightSdkError::TransferIntegerOverflow,
        PinocchioLightSdkError::Borsh,
        PinocchioLightSdkError::FewerAccountsThanSystemAccounts,
        PinocchioLightSdkError::InvalidCpiSignerAccount,
        PinocchioLightSdkError::MissingField("test".to_string()),
        PinocchioLightSdkError::OutputStateTreeIndexIsNone,
        PinocchioLightSdkError::InitAddressIsNone,
        PinocchioLightSdkError::InitWithAddressIsNone,
        PinocchioLightSdkError::InitWithAddressOutputIsNone,
        PinocchioLightSdkError::MetaMutAddressIsNone,
        PinocchioLightSdkError::MetaMutInputIsNone,
        PinocchioLightSdkError::MetaMutOutputLamportsIsNone,
        PinocchioLightSdkError::MetaMutOutputIsNone,
        PinocchioLightSdkError::MetaCloseAddressIsNone,
        PinocchioLightSdkError::MetaCloseInputIsNone,
        PinocchioLightSdkError::CpiAccountsIndexOutOfBounds(1),
        PinocchioLightSdkError::Hasher(HasherError::IntegerOverflow),
    ]
}

#[test]
fn test_error_compatibility() {
    let solana_errors = generate_all_solana_errors();
    let pinocchio_errors = generate_all_pinocchio_errors();

    // Ensure both SDKs have the same number of error variants
    assert_eq!(
        solana_errors.len(),
        pinocchio_errors.len(),
        "SDKs have different number of error variants"
    );

    // Test string representations
    for (solana_error, pinocchio_error) in solana_errors.iter().zip(pinocchio_errors.iter()) {
        assert_eq!(
            solana_error.to_string(),
            pinocchio_error.to_string(),
            "String representations differ for error variants"
        );
    }

    // Test error codes (consuming the values)
    for (solana_error, pinocchio_error) in
        solana_errors.into_iter().zip(pinocchio_errors.into_iter())
    {
        let solana_code: u32 = solana_error.into();
        let pinocchio_code: u32 = pinocchio_error.into();
        assert_eq!(
            solana_code, pinocchio_code,
            "Error codes differ for error variants"
        );
    }
}
