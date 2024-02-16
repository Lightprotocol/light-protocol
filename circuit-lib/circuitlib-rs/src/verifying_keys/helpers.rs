use groth16_solana::groth16::Groth16Verifyingkey;

use crate::{
    errors::CircuitsError,
    merkle_proof_inputs::MerkleTreeInfo,
    verifying_keys::{
        VK22_1, VK22_10, VK22_2, VK22_3, VK22_4, VK22_5, VK22_6, VK22_7, VK22_8, VK22_9, VK30_1,
        VK30_10, VK30_2, VK30_3, VK30_4, VK30_5, VK30_6, VK30_7, VK30_8, VK30_9,
    },
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
            5 => Ok(Box::new(VK22_5)),
            6 => Ok(Box::new(VK22_6)),
            7 => Ok(Box::new(VK22_7)),
            8 => Ok(Box::new(VK22_8)),
            9 => Ok(Box::new(VK22_9)),
            10 => Ok(Box::new(VK22_10)),
            _ => Err(CircuitsError::WrongNumberOfUtxos),
        },
        MerkleTreeInfo::H30 => match num_of_utxos {
            1 => Ok(Box::new(VK30_1)),
            2 => Ok(Box::new(VK30_2)),
            3 => Ok(Box::new(VK30_3)),
            4 => Ok(Box::new(VK30_4)),
            5 => Ok(Box::new(VK30_5)),
            6 => Ok(Box::new(VK30_6)),
            7 => Ok(Box::new(VK30_7)),
            8 => Ok(Box::new(VK30_8)),
            9 => Ok(Box::new(VK30_9)),
            10 => Ok(Box::new(VK30_10)),
            _ => Err(CircuitsError::WrongNumberOfUtxos),
        },
    }
}
