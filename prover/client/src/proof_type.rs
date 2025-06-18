use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProofType {
    Inclusion,
    NonInclusion,
    Combined,
    BatchAppend,
    BatchUpdate,
    BatchAddressAppend,
    BatchUpdateTest,
    BatchAppendTest,
    BatchAddressAppendTest,
}

impl Display for ProofType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ProofType::Inclusion => "inclusion",
                ProofType::NonInclusion => "non-inclusion",
                ProofType::Combined => "combined",
                ProofType::BatchAppend => "append",
                ProofType::BatchUpdate => "update",
                ProofType::BatchUpdateTest => "update-test",
                ProofType::BatchAppendTest => "append-test",
                ProofType::BatchAddressAppend => "address-append",
                ProofType::BatchAddressAppendTest => "address-append-test",
            }
        )
    }
}
