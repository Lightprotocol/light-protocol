use light_account_checks::discriminator::Discriminator as LightDiscriminator;
use light_sdk_macros::LightDiscriminator;

#[test]
fn test_anchor_discriminator() {
    #[cfg(feature = "anchor-discriminator")]
    let protocol_config_discriminator = &[96, 176, 239, 146, 1, 254, 99, 146];
    #[cfg(not(feature = "anchor-discriminator"))]
    let protocol_config_discriminator = &[254, 235, 147, 47, 205, 77, 97, 201];
    #[derive(LightDiscriminator)]
    pub struct ProtocolConfigPda {}
    assert_eq!(
        protocol_config_discriminator,
        &ProtocolConfigPda::LIGHT_DISCRIMINATOR
    );
}
