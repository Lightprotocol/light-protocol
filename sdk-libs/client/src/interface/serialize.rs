#[cfg(feature = "anchor")]
use anchor_lang::AnchorSerialize;
#[cfg(not(feature = "anchor"))]
use borsh::BorshSerialize as AnchorSerialize;

pub(crate) fn serialize_anchor_data<T: AnchorSerialize>(value: &T) -> std::io::Result<Vec<u8>> {
    let mut serialized = Vec::new();
    value.serialize(&mut serialized)?;
    Ok(serialized)
}
