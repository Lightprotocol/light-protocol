use light_sdk::compressible::CompressibleConfig;
use light_sdk::LightDiscriminator;

fn main() {
    let discriminator = CompressibleConfig::LIGHT_DISCRIMINATOR;
    println!("Expected discriminator: {:?}", discriminator);
    println!("Account data discriminator: {:?}", [55, 213, 41, 112, 43, 227, 172, 215]);
    println!("Match: {}", discriminator == [55, 213, 41, 112, 43, 227, 172, 215]);
}
