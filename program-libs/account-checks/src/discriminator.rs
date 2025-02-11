pub const ANCHOR_DISCRIMINATOR_LEN: usize = 8;

pub trait Discriminator<const T: usize> {
    const DISCRIMINATOR: [u8; T];
    fn discriminator() -> [u8; T] {
        Self::DISCRIMINATOR
    }
}
