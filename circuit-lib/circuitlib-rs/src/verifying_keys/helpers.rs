#![allow(clippy::all)]

use groth16_solana::groth16::Groth16Verifyingkey;

use crate::{
    errors::CircuitsError,
    merkle_proof_inputs::MerkleTreeInfo,
    verifying_keys::{VK22_1, VK22_2, VK22_3, VK22_4, VK22_8},
};

pub fn vk<'a>(
    mt_height: MerkleTreeInfo,
    num_of_utxos: u8,
) -> Result<Box<Groth16Verifyingkey<'a>>, CircuitsError> {
    match mt_height {
        MerkleTreeInfo::H22 => match num_of_utxos {
            1 => Ok(Box::new(VK22_1)),
            2 => Ok(Box::new(VK22_2)),
            3 => Ok(Box::new(VK22_3)),
            4 => Ok(Box::new(VK22_4)),
            8 => Ok(Box::new(VK22_8)),
            _ => Err(CircuitsError::WrongNumberOfUtxos),
        },
    }
}
