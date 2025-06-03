use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProofType {
    Inclusion,
    NonInclusion,
    Combined,
    BatchAppendWithProofs,
    BatchUpdate,
    BatchAddressAppend,
    BatchUpdateTest,
    BatchAppendWithProofsTest,
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
                ProofType::BatchAppendWithProofs => "append-with-proofs",
                ProofType::BatchUpdate => "update",
                ProofType::BatchUpdateTest => "update-test",
                ProofType::BatchAppendWithProofsTest => "append-with-proofs-test",
                ProofType::BatchAddressAppend => "addressAppend",
                ProofType::BatchAddressAppendTest => "address-append-test",
            }
        )
    }
}
