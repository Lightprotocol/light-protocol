use std::io::{self, Cursor};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    address::PackedNewAddressParams, compressed_account::PackedCompressedAccountWithMerkleContext,
    proof::ProofRpcResult,
};

pub struct LightInputs {
    pub proof: Option<ProofRpcResult>,
    pub accounts: Option<Vec<PackedCompressedAccountWithMerkleContext>>,
    pub new_addresses: Option<Vec<PackedNewAddressParams>>,
}

impl LightInputs {
    pub fn serialize(bytes: &[u8]) -> Result<Self, io::Error> {
        let mut inputs = Cursor::new(bytes);

        let proof = Option::<ProofRpcResult>::deserialize_reader(&mut inputs)?;
        let accounts = Option::<Vec<PackedCompressedAccountWithMerkleContext>>::deserialize_reader(
            &mut inputs,
        )?;
        let new_addresses = Option::<Vec<PackedNewAddressParams>>::deserialize_reader(&mut inputs)?;

        Ok(LightInputs {
            proof,
            accounts,
            new_addresses,
        })
    }

    pub fn deserialize(&self) -> Result<Vec<u8>, io::Error> {
        let mut bytes = Vec::new();
        self.proof.serialize(&mut bytes)?;
        self.accounts.serialize(&mut bytes)?;
        self.new_addresses.serialize(&mut bytes)?;
        Ok(bytes)
    }
}
