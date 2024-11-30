use std::io::{self, Cursor};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    account_meta::{pack_light_account_metas, LightAccountMeta, PackedLightAccountMeta},
    instruction_accounts::LightInstructionAccounts,
    proof::ProofRpcResult,
};

pub struct LightInstructionData {
    pub proof: Option<ProofRpcResult>,
    pub accounts: Option<Vec<LightAccountMeta>>,
}

impl LightInstructionData {
    pub fn pack(
        self,
        remaining_accounts: &mut LightInstructionAccounts,
    ) -> PackedLightInstructionData {
        PackedLightInstructionData {
            proof: self.proof,
            accounts: pack_light_account_metas(self.accounts, remaining_accounts),
        }
    }
}

pub struct PackedLightInstructionData {
    pub proof: Option<ProofRpcResult>,
    pub accounts: Option<Vec<PackedLightAccountMeta>>,
}

impl PackedLightInstructionData {
    pub fn deserialize(bytes: &[u8]) -> Result<Self, io::Error> {
        let mut inputs = Cursor::new(bytes);

        let proof = Option::<ProofRpcResult>::deserialize_reader(&mut inputs)?;
        let accounts = Option::<Vec<PackedLightAccountMeta>>::deserialize_reader(&mut inputs)?;

        Ok(PackedLightInstructionData { proof, accounts })
    }

    pub fn serialize(&self) -> Result<Vec<u8>, io::Error> {
        let mut bytes = Vec::new();
        self.proof.serialize(&mut bytes)?;
        self.accounts.serialize(&mut bytes)?;
        Ok(bytes)
    }
}
