#[cfg(feature = "anchor")]
#[test]
fn test_discriminator() {
    use anchor_compressible::UserRecord;
    use anchor_lang::Discriminator;
    use light_sdk::LightDiscriminator;
    let light_discriminator = UserRecord::LIGHT_DISCRIMINATOR;
    println!("light discriminator: {:?}", light_discriminator);

    let anchor_discriminator = UserRecord::LIGHT_DISCRIMINATOR;

    println!("Anchor discriminator: {:?}", anchor_discriminator);
    println!("Match: {}", light_discriminator == anchor_discriminator);

    assert_eq!(light_discriminator, anchor_discriminator);
}
