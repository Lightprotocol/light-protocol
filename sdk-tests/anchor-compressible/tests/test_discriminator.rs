#[test]
fn test_discriminator() {
    use anchor_compressible::UserRecord;
    use anchor_lang::Discriminator;
    use light_sdk::LightDiscriminator;

    // anchor
    let light_discriminator = UserRecord::DISCRIMINATOR;
    println!("light discriminator: {:?}", light_discriminator);

    // ours (should be anchor compatible.)
    let anchor_discriminator = UserRecord::LIGHT_DISCRIMINATOR;

    println!("Anchor discriminator: {:?}", anchor_discriminator);
    println!("Match: {}", light_discriminator == anchor_discriminator);

    assert_eq!(light_discriminator, anchor_discriminator);
}
