use std::io::{self, Cursor};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{account_info::LightAccountInfo, proof::ProofRpcResult};

pub struct LightInputs {
    pub proof: Option<ProofRpcResult>,
    pub accounts: Option<Vec<LightAccountInfo>>,
}

impl LightInputs {
    pub fn deserialize(bytes: &[u8]) -> Result<Self, io::Error> {
        let mut inputs = Cursor::new(bytes);

        let proof = Option::<ProofRpcResult>::deserialize_reader(&mut inputs)?;
        let accounts = Option::<Vec<LightAccountInfo>>::deserialize_reader(&mut inputs)?;

        Ok(LightInputs { proof, accounts })
    }

    pub fn serialize(&self) -> Result<Vec<u8>, io::Error> {
        let mut bytes = Vec::new();
        self.proof.serialize(&mut bytes)?;
        self.accounts.serialize(&mut bytes)?;
        Ok(bytes)
    }
}
