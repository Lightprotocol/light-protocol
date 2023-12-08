use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{
    prelude::*,
    solana_program::keccak::{hash, hashv},
};
use light_hasher::{Hasher, Poseidon};
// use light_utils::hash_and_truncate_to_circuit;
#[cfg(all(target_os = "solana", feature = "custom-heap"))]
use crate::light_transaction::custom_heap;
use num_bigint::BigUint;
use std::default::Default;
pub const DEFAULT_PUBKEY: [u8; 32] = [
    41, 23, 97, 0, 234, 169, 98, 189, 193, 254, 108, 101, 77, 106, 60, 19, 14, 150, 164, 209, 22,
    139, 51, 132, 139, 137, 125, 197, 2, 130, 1, 51,
];
pub const DEFAULT_UTXO_HASH: [u8; 32] = [
    6, 134, 74, 221, 170, 213, 21, 229, 100, 130, 25, 62, 161, 231, 45, 154, 122, 119, 168, 135,
    80, 19, 148, 64, 46, 218, 152, 255, 117, 244, 41, 10,
];
#[derive(Clone, Debug, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
pub struct Utxo {
    pub version: u64,
    pub pool_type: u64,
    pub amounts: [u64; 2],
    // TODO: make option
    pub spl_asset_mint: Option<Pubkey>,
    pub owner: [u8; 32],
    pub blinding: [u8; 32],
    pub data_hash: [u8; 32],
    pub meta_hash: [u8; 32],
    pub address: [u8; 32],
    pub message: Option<Vec<u8>>,
}

impl<'a> Default for Utxo {
    fn default() -> Self {
        Utxo {
            version: 0,
            pool_type: 0,
            amounts: [0; 2],
            spl_asset_mint: None,
            owner: DEFAULT_PUBKEY,
            blinding: [0; 32],
            data_hash: [0; 32],
            meta_hash: [0; 32],
            address: [0; 32],
            message: None,
        }
    }
}

impl Utxo {
    pub fn new(
        version: u64,
        pool_type: u64,
        amounts: [u64; 2],
        spl_asset_mint: Option<Pubkey>,
        owner: [u8; 32],
        blinding: [u8; 32],
        data_hash: [u8; 32],
        meta_hash: [u8; 32],
        address: [u8; 32],
        message: Option<Vec<u8>>,
    ) -> Self {
        Self {
            version,
            pool_type,
            amounts,
            spl_asset_mint,
            owner,
            blinding,
            data_hash,
            meta_hash,
            address,
            message,
        }
    }

    pub fn compute_amount_hash(&self) -> Result<[u8; 32]> {
        let hash = Poseidon::hashv(&[
            &self.amounts[0].to_be_bytes(),
            &self.amounts[1].to_be_bytes(),
        ])
        .unwrap();
        Ok(hash)
    }

    pub fn compute_asset_hash(&self) -> Result<[u8; 32]> {
        let spl_circuit = match self.spl_asset_mint {
            Some(mint) => hash_and_truncate_to_circuit(&[mint.to_bytes().as_slice()]),
            None => [0u8; 32],
        };
        let hash = Poseidon::hashv(&[
            &BigUint::parse_bytes(
                b"6686672797465227418401714772753289406522066866583537086457438811846503839916",
                10,
            )
            .unwrap()
            .to_bytes_be()
            .as_slice(),
            &spl_circuit,
        ])
        .unwrap();
        Ok(hash)
    }

    pub fn hash(&self) -> Result<[u8; 32]> {
        // If there is data it is a program utxo which is owned by a Program
        let owner = if self.data_hash == [0u8; 32] {
            self.owner
        } else {
            hash_and_truncate_to_circuit(&[self.owner.as_slice()])
        };
        #[cfg(all(target_os = "solana", feature = "custom-heap"))]
        let pos = custom_heap::get_heap_pos();
        msg!("version: {:?}", self.version);
        msg!("amount hash {:?}", self.compute_amount_hash().unwrap());
        msg!("owner: {:?}", owner);
        msg!("blinding: {:?}", self.blinding);
        msg!("asset hash: {:?}", self.compute_asset_hash().unwrap());
        msg!("data hash: {:?}", self.data_hash);
        msg!("pool type: {:?}", self.pool_type);
        msg!("meta hash: {:?}", self.meta_hash);
        msg!("address: {:?}", self.address);
        #[cfg(all(target_os = "solana", feature = "custom-heap"))]
        custom_heap::free_heap(pos);
        let hash = Poseidon::hashv(&[
            BigUint::from(self.version).to_bytes_be().as_slice(),
            self.compute_amount_hash().unwrap().as_slice(),
            owner.as_slice(),
            self.blinding.as_slice(),
            self.compute_asset_hash().unwrap().as_slice(),
            self.data_hash.as_slice(),
            BigUint::from(self.pool_type).to_bytes_be().as_slice(),
            self.meta_hash.as_slice(),
            self.address.as_slice(),
        ])
        .unwrap();
        msg!("hash {:?}", hash);
        Ok(hash)
    }

    /// the utxo hash which is used as the public input for the zkp must have blinding zero
    /// the utxo hash which is inserted into the state tree must have the updated blinding
    pub fn update_blinding(&mut self, merkle_tree_pda: Pubkey, index_of_leaf: usize) -> Result<()> {
        self.blinding = Poseidon::hashv(&[
            &hash(merkle_tree_pda.to_bytes().as_slice()).to_bytes()[0..30],
            index_of_leaf.to_le_bytes().as_slice(),
        ])
        .unwrap();
        Ok(())
    }
}

pub fn hash_and_truncate_to_circuit(data: &[&[u8]]) -> [u8; 32] {
    let hashed_data = data.iter().map(|d| hash(d).to_bytes()).collect::<Vec<_>>();
    let truncated_data = &hashv(
        hashed_data
            .iter()
            .map(|d| &d[..])
            .collect::<Vec<_>>()
            .as_slice(),
    )
    .to_bytes()[0..30];
    let hash = Poseidon::hash(truncated_data).unwrap();
    hash
}
#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    // test implementation for idl generation this needs to be defined as account in anchor program
    #[derive(Clone, Debug, PartialEq, Eq, AnchorDeserialize, AnchorSerialize, Default)]
    #[allow(non_camel_case_types)]
    pub struct u256 {
        pub data: [u8; 32],
    }

    // test implementation for idl generation this needs to be defined as account in anchor program
    pub fn from_transfer_output_utxo<'a>(utxo: TransferOutputUtxo) -> Utxo {
        let mut owner = utxo.owner.data.clone();
        owner.reverse();
        Utxo {
            version: 0,
            pool_type: 0,
            amounts: utxo.amounts,
            spl_asset_mint: utxo.spl_asset_mint,
            owner,
            blinding: [0u8; 32],
            data_hash: [0u8; 32],
            meta_hash: utxo.meta_hash.unwrap_or(u256 { data: [0u8; 32] }).data,
            address: utxo.address.unwrap_or(u256 { data: [0u8; 32] }).data,
            message: None,
        }
    }

    // test implementation for idl generation this needs to be defined as account in anchor program
    #[derive(Clone, Debug, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
    pub struct TransferOutputUtxo {
        pub amounts: [u64; 2],
        pub spl_asset_mint: Option<Pubkey>,
        pub owner: u256,
        pub meta_hash: Option<u256>,
        pub address: Option<u256>,
    }

    #[ignore]
    #[test]
    fn poseidon_hash_parsing() {
        let one = [1u8; 1];
        let hash = Poseidon::hashv(&[one.as_slice()]).unwrap();
        println!("{:?}", hash);
        let mut one = [0u8; 32];
        one[31] = 1;
        let hash = Poseidon::hashv(&[one.as_slice()]).unwrap();
        println!("{:?}", hash);
    }

    #[test]
    fn utxo_transfer_output_utxo_functional() -> Result<()> {
        let sol_amount = 1u64;
        let token_amount = 2u64;
        let mint = Pubkey::from_str("ycrF6Bw3doNPMSDmZM1rxNHimD2bwq1UFmifMCzbjAe").unwrap();
        let owner = [
            32, 29, 80, 210, 12, 55, 172, 224, 206, 72, 234, 251, 4, 214, 215, 140, 183, 183, 99,
            27, 207, 3, 220, 89, 216, 44, 41, 209, 140, 56, 131, 67,
        ];
        let blinding = [2u8; 32];
        let data_hash = [0u8; 32];
        let meta_hash = [0u8; 32];
        let address = [0u8; 32];
        let mut utxo = Utxo::new(
            0,
            0,
            [sol_amount, token_amount],
            Some(mint),
            owner,
            blinding,
            data_hash,
            meta_hash,
            address,
            None,
        );
        let amount_hash = utxo.compute_amount_hash()?;

        let reference = BigUint::parse_bytes(
            b"7853200120776062878684798364095072458815029376092732009249414926327459813530",
            10,
        )
        .unwrap();
        assert_eq!(BigUint::from_bytes_be(&amount_hash), reference);
        let asset_hash = utxo.compute_asset_hash()?;
        let reference = BigUint::parse_bytes(
            b"9340065326044129008197171558760186932973940749536260832525108848644208614347",
            10,
        )
        .unwrap();
        assert_eq!(BigUint::from_bytes_be(&asset_hash), reference);

        let hash = utxo.hash()?;
        let reference_hash = [
            38, 242, 134, 116, 20, 185, 146, 113, 93, 43, 96, 138, 136, 18, 54, 127, 230, 238, 6,
            154, 117, 234, 29, 80, 198, 180, 111, 7, 45, 187, 137, 244,
        ];
        assert_eq!(hash, reference_hash);
        utxo.update_blinding(mint, 2).unwrap();
        let hash = utxo.hash()?;
        assert_ne!(hash, reference_hash);

        let transfer_output_utxo = TransferOutputUtxo {
            amounts: [sol_amount, token_amount],
            spl_asset_mint: Some(mint),
            owner: u256 { data: owner },
            meta_hash: None,
            address: None,
        };
        let mut from_transfer_output_utxo = from_transfer_output_utxo(transfer_output_utxo);
        from_transfer_output_utxo.update_blinding(mint, 2).unwrap();
        assert_eq!(utxo, from_transfer_output_utxo);
        println!("{:?}", utxo);
        println!("{:?}", utxo.try_to_vec().unwrap());

        Ok(())
    }

    #[test]
    fn default_utxo_is_equal_to_filling_utxo() {
        let default_utxo = Utxo::default();
        let reference = [
            6, 134, 74, 221, 170, 213, 21, 229, 100, 130, 25, 62, 161, 231, 45, 154, 122, 119, 168,
            135, 80, 19, 148, 64, 46, 218, 152, 255, 117, 244, 41, 10,
        ];
        assert_eq!(default_utxo.hash().unwrap(), reference);
    }
}
