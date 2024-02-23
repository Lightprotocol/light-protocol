#![allow(clippy::all)]

use groth16_solana::groth16::Groth16Verifyingkey;

use crate::{
    errors::CircuitsError,
    merkle_proof_inputs::MerkleTreeInfo,
    verifying_keys::{VK26_1, VK26_2, VK26_3, VK26_4, VK26_8},
};

pub fn vk<'a>(
    mt_height: MerkleTreeInfo,
    num_of_utxos: u8,
) -> Result<Box<Groth16Verifyingkey<'a>>, CircuitsError> {
    match mt_height {
        MerkleTreeInfo::H26 => match num_of_utxos {
            1 => Ok(Box::new(VK26_1)),
            2 => Ok(Box::new(VK26_2)),
            3 => Ok(Box::new(VK26_3)),
            4 => Ok(Box::new(VK26_4)),
            8 => Ok(Box::new(VK26_8)),
            _ => Err(CircuitsError::WrongNumberOfUtxos),
        },
    }
}
