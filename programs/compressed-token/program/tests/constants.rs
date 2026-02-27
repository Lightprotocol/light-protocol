use light_compressed_token::RENT_SPONSOR_V1;
use light_compressible::config::CompressibleConfig;

#[test]
fn rent_sponsor_v1_matches_compressible_config() {
    let expected = CompressibleConfig::light_token_v1_rent_sponsor_pda();
    assert_eq!(
        RENT_SPONSOR_V1,
        expected.to_bytes(),
        "RENT_SPONSOR_V1 in the program must match light_token_v1_rent_sponsor_pda()"
    );
}
