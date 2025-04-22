pub const DISCRIMINATOR_LEN: usize = 8;

pub trait Discriminator {
    const DISCRIMINATOR: [u8; 8];
    const DISCRIMINATOR_SLICE: &'static [u8];
    fn discriminator() -> [u8; 8] {
        Self::DISCRIMINATOR
    }
}
