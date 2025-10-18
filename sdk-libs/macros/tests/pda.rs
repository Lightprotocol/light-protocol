use std::str::FromStr;

use light_macros::derive_light_cpi_signer;
use light_sdk_types::CpiSigner;
use solana_pubkey::Pubkey;

#[test]
fn test_compute_pda_basic() {
    // Test with a known program ID using fixed "cpi_authority" seed
    const RESULT: CpiSigner =
        derive_light_cpi_signer!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

    // Verify the result has valid fields
    assert_eq!(RESULT.program_id.len(), 32);
    assert_eq!(RESULT.cpi_signer.len(), 32);

    // Verify this matches runtime computation
    let runtime_result = Pubkey::find_program_address(
        &[b"cpi_authority"],
        &Pubkey::from_str("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7").unwrap(),
    );

    assert_eq!(RESULT.cpi_signer, runtime_result.0.to_bytes());
    assert_eq!(RESULT.bump, runtime_result.1);
}

#[test]
fn test_cpi_signer() {
    // Test that the macro can be used in const contexts
    const PDA_RESULT: CpiSigner =
        derive_light_cpi_signer!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

    // Extract individual components in const context
    const PROGRAM_ID: [u8; 32] = PDA_RESULT.program_id;
    const CPI_SIGNER: [u8; 32] = PDA_RESULT.cpi_signer;
    const BUMP: u8 = PDA_RESULT.bump;

    // Verify they're valid
    assert_eq!(
        PROGRAM_ID,
        light_macros::pubkey_array!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7")
    );
    assert_eq!(
        CPI_SIGNER,
        [
            251, 179, 40, 117, 16, 92, 174, 133, 181, 180, 68, 118, 7, 237, 191, 225, 69, 39, 191,
            180, 35, 145, 28, 164, 4, 35, 191, 209, 82, 122, 38, 117
        ]
    );
    assert_eq!(BUMP, 255);
}

#[test]
fn test_cpi_signer_2() {
    // Test that the macro can be used in const contexts
    const PDA_RESULT: CpiSigner =
        derive_light_cpi_signer!("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");

    // Extract individual components in const context
    const PROGRAM_ID: [u8; 32] = PDA_RESULT.program_id;
    const CPI_SIGNER: [u8; 32] = PDA_RESULT.cpi_signer;
    const BUMP: u8 = PDA_RESULT.bump;

    // Verify they're valid
    assert_eq!(
        PROGRAM_ID,
        light_macros::pubkey_array!("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq")
    );
    assert_eq!(
        CPI_SIGNER,
        [
            20, 12, 243, 109, 120, 11, 194, 48, 169, 64, 170, 103, 246, 66, 224, 151, 74, 116, 57,
            84, 0, 180, 16, 126, 175, 149, 24, 207, 85, 137, 3, 207
        ]
    );
    assert_eq!(BUMP, 255);
}
