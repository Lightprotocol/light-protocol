pub const DISCRIMINATOR_LEN: usize = 8;

pub trait Discriminator {
    const LIGHT_DISCRIMINATOR: [u8; 8];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8];
    fn discriminator() -> [u8; 8] {
        Self::LIGHT_DISCRIMINATOR
    }
}
