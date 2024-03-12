#![allow(clippy::all)]

use groth16_solana::groth16::Groth16Verifyingkey;

use crate::{
    errors::CircuitsError,
    inclusion::merkle_tree_info::MerkleTreeInfo,
    verifying_keys::{VK_i_26_1, VK_i_26_2, VK_i_26_3, VK_i_26_4, VK_i_26_8},
};

pub fn vk<'a>(
    mt_height: MerkleTreeInfo,
    num_of_utxos: u8,
) -> Result<Box<Groth16Verifyingkey<'a>>, CircuitsError> {
    match mt_height {
        MerkleTreeInfo::H26 => match num_of_utxos {
            1 => Ok(Box::new(VK_i_26_1)),
            2 => Ok(Box::new(VK_i_26_2)),
            3 => Ok(Box::new(VK_i_26_3)),
            4 => Ok(Box::new(VK_i_26_4)),
            8 => Ok(Box::new(VK_i_26_8)),
            _ => Err(CircuitsError::WrongNumberOfUtxos),
        },
    }
}
