#[derive(Debug, PartialEq, Eq)]
pub enum CircuitType {
    Combined,
    Inclusion,
    NonInclusion,
    BatchAppendWithProofs,
    BatchUpdate,
    BatchAddressAppend,
}

impl CircuitType {
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        match self {
            Self::Combined => "combined".to_string(),
            Self::Inclusion => "inclusion".to_string(),
            Self::NonInclusion => "non-inclusion".to_string(),
            Self::BatchAppendWithProofs => "append-with-proofs".to_string(),
            Self::BatchUpdate => "update".to_string(),
            Self::BatchAddressAppend => "address-append".to_string(),
        }
    }
}
