use std::io::{self, Cursor};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::account_meta::LightAccountMeta;

use light_client::rpc::ProofRpcResult;

pub struct LightInstructionData {
    pub proof: Option<ProofRpcResult>,
    pub accounts: Option<Vec<LightAccountMeta>>,
}

impl LightInstructionData {
    pub fn deserialize(bytes: &[u8]) -> Result<Self, io::Error> {
        let mut inputs = Cursor::new(bytes);

        let proof = Option::<ProofRpcResult>::deserialize_reader(&mut inputs)?;
        let accounts = Option::<Vec<LightAccountMeta>>::deserialize_reader(&mut inputs)?;

        Ok(LightInstructionData { proof, accounts })
    }

    pub fn serialize(&self) -> Result<Vec<u8>, io::Error> {
        let mut bytes = Vec::new();
        self.proof.serialize(&mut bytes)?;
        self.accounts.serialize(&mut bytes)?;
        Ok(bytes)
    }
}
