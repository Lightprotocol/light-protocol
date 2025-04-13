use std::collections::HashMap;

use light_hasher::{Hasher, Poseidon};

use crate::{
    address::pack_account,
    hash_to_bn254_field_size_be,
    instruction_data::{
        data::OutputCompressedAccountWithPackedContext, zero_copy::ZCompressedAccount,
    },
    AnchorDeserialize, AnchorSerialize, CompressedAccountError, Pubkey, TreeType,
};

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedCompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: PackedMerkleContext,
    /// Index of root used in inclusion validity proof.
    pub root_index: u16,
    pub read_only: bool,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InCompressedAccountWithMerkleContext {
    pub compressed_account: InCompressedAccount,
    pub merkle_context: MerkleContext,
}

impl From<CompressedAccount> for InCompressedAccount {
    fn from(value: CompressedAccount) -> Self {
        let data = value.data.unwrap_or_default();
        InCompressedAccount {
            owner: value.owner,
            lamports: value.lamports,
            address: value.address,
            discriminator: data.discriminator,
            data_hash: data.data_hash,
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedInCompressedAccountWithMerkleContext {
    pub compressed_account: InCompressedAccount,
    pub merkle_context: PackedMerkleContext,
    /// Index of root used in inclusion validity proof.
    pub root_index: u16,
}

impl From<PackedCompressedAccountWithMerkleContext> for PackedInCompressedAccountWithMerkleContext {
    fn from(value: PackedCompressedAccountWithMerkleContext) -> Self {
        Self {
            compressed_account: value.compressed_account.into(),
            merkle_context: value.merkle_context,
            root_index: value.root_index,
        }
    }
}

impl From<CompressedAccountWithMerkleContext> for InCompressedAccountWithMerkleContext {
    fn from(value: CompressedAccountWithMerkleContext) -> Self {
        Self {
            compressed_account: value.compressed_account.into(),
            merkle_context: value.merkle_context,
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: MerkleContext,
}

impl CompressedAccountWithMerkleContext {
    pub fn hash(&self) -> Result<[u8; 32], CompressedAccountError> {
        self.compressed_account.hash(
            &self.merkle_context.merkle_tree_pubkey,
            &self.merkle_context.leaf_index,
            self.merkle_context.tree_type == TreeType::StateV2,
        )
    }
}

impl CompressedAccountWithMerkleContext {
    pub fn into_read_only(
        &self,
        root_index: Option<u16>,
    ) -> Result<ReadOnlyCompressedAccount, CompressedAccountError> {
        let account_hash = self.hash()?;
        let merkle_context = if root_index.is_none() {
            let mut merkle_context = self.merkle_context;
            merkle_context.prove_by_index = true;
            merkle_context
        } else {
            self.merkle_context
        };
        Ok(ReadOnlyCompressedAccount {
            account_hash,
            merkle_context,
            root_index: root_index.unwrap_or_default(),
        })
    }

    pub fn pack(
        &self,
        root_index: Option<u16>,
        remaining_accounts: &mut HashMap<Pubkey, usize>,
    ) -> Result<PackedCompressedAccountWithMerkleContext, CompressedAccountError> {
        Ok(PackedCompressedAccountWithMerkleContext {
            compressed_account: self.compressed_account.clone(),
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: pack_account(
                    &self.merkle_context.merkle_tree_pubkey,
                    remaining_accounts,
                ),
                nullifier_queue_pubkey_index: pack_account(
                    &self.merkle_context.nullifier_queue_pubkey,
                    remaining_accounts,
                ),
                leaf_index: self.merkle_context.leaf_index,
                prove_by_index: root_index.is_none(),
            },
            root_index: root_index.unwrap_or_default(),
            read_only: false,
        })
    }
}
#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct ReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],
    pub merkle_context: MerkleContext,
    pub root_index: u16,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct MerkleContext {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub leaf_index: u32,
    pub prove_by_index: bool,
    pub tree_type: TreeType,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Default)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: u32,
    pub prove_by_index: bool,
}

pub fn pack_compressed_accounts(
    compressed_accounts: &[CompressedAccountWithMerkleContext],
    root_indices: &[Option<u16>],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<PackedCompressedAccountWithMerkleContext> {
    compressed_accounts
        .iter()
        .zip(root_indices.iter())
        .map(|(x, root_index)| {
            let mut merkle_context = x.merkle_context;
            let root_index = if let Some(root) = root_index {
                *root
            } else {
                merkle_context.prove_by_index = true;
                0
            };

            PackedCompressedAccountWithMerkleContext {
                compressed_account: x.compressed_account.clone(),
                merkle_context: pack_merkle_context(&[merkle_context], remaining_accounts)[0],
                root_index,
                read_only: false,
            }
        })
        .collect::<Vec<_>>()
}

pub fn pack_output_compressed_accounts(
    compressed_accounts: &[CompressedAccount],
    merkle_trees: &[Pubkey],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<OutputCompressedAccountWithPackedContext> {
    compressed_accounts
        .iter()
        .zip(merkle_trees.iter())
        .map(|(x, tree)| OutputCompressedAccountWithPackedContext {
            compressed_account: x.clone(),
            merkle_tree_index: pack_account(tree, remaining_accounts),
        })
        .collect::<Vec<_>>()
}

pub fn pack_merkle_context(
    merkle_context: &[MerkleContext],
    remaining_accounts: &mut HashMap<Pubkey, usize>,
) -> Vec<PackedMerkleContext> {
    merkle_context
        .iter()
        .map(|merkle_context| PackedMerkleContext {
            leaf_index: merkle_context.leaf_index,
            merkle_tree_pubkey_index: pack_account(
                &merkle_context.merkle_tree_pubkey,
                remaining_accounts,
            ),
            nullifier_queue_pubkey_index: pack_account(
                &merkle_context.nullifier_queue_pubkey,
                remaining_accounts,
            ),
            prove_by_index: merkle_context.prove_by_index,
        })
        .collect::<Vec<_>>()
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccount {
    pub owner: Pubkey,
    pub lamports: u64,
    pub address: Option<[u8; 32]>,
    pub data: Option<CompressedAccountData>,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InCompressedAccount {
    pub owner: Pubkey,
    pub lamports: u64,
    pub discriminator: [u8; 8],
    pub data_hash: [u8; 32],
    pub address: Option<[u8; 32]>,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountData {
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}

pub fn hash_with_hashed_values(
    lamports: &u64,
    address: Option<&[u8]>,
    data: Option<(&[u8], &[u8])>,
    owner_hashed: &[u8; 32],
    merkle_tree_hashed: &[u8; 32],
    leaf_index: &u32,
    is_batched: bool,
) -> Result<[u8; 32], CompressedAccountError> {
    // TODO: replace with array
    let capacity = 3
        + std::cmp::min(*lamports, 1) as usize
        + address.is_some() as usize
        + data.is_some() as usize * 2;
    let mut vec: Vec<&[u8]> = Vec::with_capacity(capacity);
    vec.push(owner_hashed.as_slice());

    // leaf index and merkle tree pubkey are used to make every compressed account hash unique
    let mut leaf_index_bytes = [0u8; 32];
    if is_batched {
        leaf_index_bytes[28..].copy_from_slice(&leaf_index.to_be_bytes());
    } else {
        leaf_index_bytes[28..].copy_from_slice(&leaf_index.to_le_bytes());
    };
    vec.push(leaf_index_bytes.as_slice());

    vec.push(merkle_tree_hashed.as_slice());

    // Lamports are only hashed if non-zero to save CU.
    // For safety, we prefix lamports with 1 in 1 byte.
    // Thus, even if the discriminator has the same value as the lamports, the hash will be different.
    let mut lamports_bytes = [0u8; 32];
    if *lamports != 0 {
        if is_batched {
            lamports_bytes[24..].copy_from_slice(&lamports.to_be_bytes());
        } else {
            lamports_bytes[24..].copy_from_slice(&lamports.to_le_bytes());
        };
        lamports_bytes[23] = 1;

        vec.push(lamports_bytes.as_slice());
    }

    if let Some(address) = address {
        vec.push(address);
    }

    let mut discriminator_bytes = [0u8; 32];
    if let Some((discriminator, data_hash)) = data {
        discriminator_bytes[24..].copy_from_slice(discriminator);
        discriminator_bytes[23] = 2;
        vec.push(&discriminator_bytes);
        vec.push(data_hash);
    }

    Ok(Poseidon::hashv(&vec)?)
}

/// Hashing scheme:
/// H(owner || leaf_index || merkle_tree_pubkey || lamports || address || data.discriminator || data.data_hash)
impl CompressedAccount {
    pub fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError> {
        hash_with_hashed_values(
            &self.lamports,
            self.address.as_ref().map(|x| x.as_slice()),
            self.data
                .as_ref()
                .map(|x| (x.discriminator.as_slice(), x.data_hash.as_slice())),
            owner_hashed,
            merkle_tree_hashed,
            leaf_index,
            is_batched,
        )
    }

    pub fn hash(
        &self,
        &merkle_tree_pubkey: &Pubkey,
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError> {
        let hashed_mt = hash_to_bn254_field_size_be(merkle_tree_pubkey.as_ref());
        self.hash_with_hashed_values(
            &hash_to_bn254_field_size_be(self.owner.as_ref()),
            &hashed_mt,
            leaf_index,
            is_batched,
        )
    }
}

/// Hashing scheme:
/// H(owner || leaf_index || merkle_tree_pubkey || lamports || address || data.discriminator || data.data_hash)
impl ZCompressedAccount<'_> {
    pub fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError> {
        hash_with_hashed_values(
            &(self.lamports.into()),
            self.address.as_ref().map(|x| x.as_slice()),
            self.data
                .as_ref()
                .map(|x| (x.discriminator.as_slice(), x.data_hash.as_slice())),
            owner_hashed,
            merkle_tree_hashed,
            leaf_index,
            is_batched,
        )
    }
    pub fn hash(
        &self,
        &merkle_tree_pubkey: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError> {
        self.hash_with_hashed_values(
            &hash_to_bn254_field_size_be(&self.owner.to_bytes()),
            &hash_to_bn254_field_size_be(merkle_tree_pubkey.as_slice()),
            leaf_index,
            is_batched,
        )
    }
}

#[cfg(not(feature = "pinocchio"))]
#[cfg(test)]
mod tests {
    use light_hasher::Poseidon;
    use light_zero_copy::borsh::Deserialize;
    use num_bigint::BigUint;
    use rand::Rng;

    use super::*;
    /// Tests:
    /// 1. functional with all inputs set
    /// 2. no data
    /// 3. no address
    /// 4. no address and no lamports
    /// 5. no address and no data
    /// 6. no address, no data, no lamports
    #[test]
    fn test_compressed_account_hash() {
        let owner = Pubkey::new_unique();
        let address = [1u8; 32];
        let data = CompressedAccountData {
            discriminator: [1u8; 8],
            data: vec![2u8; 32],
            data_hash: [3u8; 32],
        };
        let lamports = 100;
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: Some(address),
            data: Some(data.clone()),
        };
        let merkle_tree_pubkey = Pubkey::new_unique();
        let leaf_index: u32 = 1;
        let hash = compressed_account
            .hash(&merkle_tree_pubkey, &leaf_index, false)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes()).as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes()).as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
            address.as_slice(),
            [&[2u8], data.discriminator.as_slice()].concat().as_slice(),
            &data.data_hash,
        ])
        .unwrap();
        assert_eq!(hash, hash_manual);
        assert_eq!(hash.len(), 32);

        // no data
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: Some(address),
            data: None,
        };
        let no_data_hash = compressed_account
            .hash(&merkle_tree_pubkey, &leaf_index, false)
            .unwrap();

        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes()).as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes()).as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
            address.as_slice(),
        ])
        .unwrap();
        assert_eq!(no_data_hash, hash_manual);
        assert_ne!(hash, no_data_hash);

        // no address
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: None,
            data: Some(data.clone()),
        };
        let no_address_hash = compressed_account
            .hash(&merkle_tree_pubkey, &leaf_index, false)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes()).as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes()).as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
            [&[2u8], data.discriminator.as_slice()].concat().as_slice(),
            &data.data_hash,
        ])
        .unwrap();
        assert_eq!(no_address_hash, hash_manual);
        assert_ne!(hash, no_address_hash);
        assert_ne!(no_data_hash, no_address_hash);

        // no address no lamports
        let compressed_account = CompressedAccount {
            owner,
            lamports: 0,
            address: None,
            data: Some(data.clone()),
        };
        let no_address_no_lamports_hash = compressed_account
            .hash(&merkle_tree_pubkey, &leaf_index, false)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes()).as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes()).as_slice(),
            [&[2u8], data.discriminator.as_slice()].concat().as_slice(),
            &data.data_hash,
        ])
        .unwrap();
        assert_eq!(no_address_no_lamports_hash, hash_manual);
        assert_ne!(hash, no_address_no_lamports_hash);
        assert_ne!(no_data_hash, no_address_no_lamports_hash);
        assert_ne!(no_address_hash, no_address_no_lamports_hash);

        // no address and no data
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: None,
            data: None,
        };
        let no_address_no_data_hash = compressed_account
            .hash(&merkle_tree_pubkey, &leaf_index, false)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes()).as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes()).as_slice(),
            [&[1u8], lamports.to_le_bytes().as_slice()]
                .concat()
                .as_slice(),
        ])
        .unwrap();
        assert_eq!(no_address_no_data_hash, hash_manual);
        assert_ne!(hash, no_address_no_data_hash);
        assert_ne!(no_data_hash, no_address_no_data_hash);
        assert_ne!(no_address_hash, no_address_no_data_hash);
        assert_ne!(no_address_no_lamports_hash, no_address_no_data_hash);

        // no address, no data, no lamports
        let compressed_account = CompressedAccount {
            owner,
            lamports: 0,
            address: None,
            data: None,
        };
        let no_address_no_data_no_lamports_hash = compressed_account
            .hash(&merkle_tree_pubkey, &leaf_index, false)
            .unwrap();
        let hash_manual = Poseidon::hashv(&[
            hash_to_bn254_field_size_be(&owner.to_bytes()).as_slice(),
            leaf_index.to_le_bytes().as_slice(),
            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes()).as_slice(),
        ])
        .unwrap();
        assert_eq!(no_address_no_data_no_lamports_hash, hash_manual);
        assert_ne!(no_address_no_data_hash, no_address_no_data_no_lamports_hash);
        assert_ne!(hash, no_address_no_data_no_lamports_hash);
        assert_ne!(no_data_hash, no_address_no_data_no_lamports_hash);
        assert_ne!(no_address_hash, no_address_no_data_no_lamports_hash);
        assert_ne!(
            no_address_no_lamports_hash,
            no_address_no_data_no_lamports_hash
        );
    }

    #[test]
    fn reference() {
        let owner = Pubkey::new_from_array([
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        let address = [
            0, 21, 245, 15, 61, 157, 224, 84, 69, 48, 190, 72, 43, 19, 47, 25, 14, 118, 20, 147,
            40, 141, 175, 33, 233, 58, 36, 179, 73, 137, 84, 99,
        ];

        let data = CompressedAccountData {
            discriminator: [0, 0, 0, 0, 0, 0, 0, 1],
            data: vec![2u8; 31],
            data_hash: Poseidon::hash(&[vec![2u8; 31], vec![0u8]].concat()).unwrap(),
        };
        let lamports = 100;
        let compressed_account = CompressedAccount {
            owner,
            lamports,
            address: Some(address),
            data: Some(data.clone()),
        };
        let bytes: Vec<u8> = compressed_account.try_to_vec().unwrap();
        let merkle_tree_pubkey = Pubkey::new_from_array([
            0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);

        let leaf_index: u32 = 1;
        let hash = compressed_account
            .hash(&merkle_tree_pubkey, &leaf_index, false)
            .unwrap();
        let (z_account, _) = ZCompressedAccount::zero_copy_at(&bytes).unwrap();
        let z_hash = z_account
            .hash(&merkle_tree_pubkey.to_bytes(), &leaf_index, false)
            .unwrap();
        let manual_hash = {
            let mut hasher = light_poseidon::Poseidon::<Fr>::new_circom(7).unwrap();
            use ark_bn254::Fr;
            use ark_ff::{BigInteger, PrimeField};
            let hashed_owner = hash_to_bn254_field_size_be(&owner.to_bytes());
            let owner = Fr::from_be_bytes_mod_order(hashed_owner.as_slice());
            let leaf_index = Fr::from_be_bytes_mod_order(leaf_index.to_le_bytes().as_ref());
            let hashed_mt = hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes());
            let merkle_tree_pubkey = Fr::from_be_bytes_mod_order(hashed_mt.as_slice());
            let lamports = Fr::from_be_bytes_mod_order(lamports.to_le_bytes().as_ref())
                + Fr::from_be_bytes_mod_order(&[1u8, 0, 0, 0, 0, 0, 0, 0, 0]);
            let address = Fr::from_be_bytes_mod_order(address.as_slice());
            let discriminator = Fr::from_be_bytes_mod_order(data.discriminator.as_ref());
            let domain_separated_discriminator =
                Fr::from_be_bytes_mod_order(&[2, 0, 0, 0, 0, 0, 0, 0, 0]);
            let data_discriminator = discriminator + domain_separated_discriminator;
            use light_poseidon::PoseidonHasher;
            let inputs = [
                owner,
                leaf_index,
                merkle_tree_pubkey,
                lamports,
                address,
                data_discriminator,
                Fr::from_be_bytes_mod_order(data.data_hash.as_ref()),
            ];
            hasher.hash(&inputs).unwrap().into_bigint().to_bytes_be()
        };
        assert_eq!(hash.to_vec(), manual_hash);
        assert_eq!(z_hash.to_vec(), manual_hash);
        assert_eq!(hash.len(), 32);

        let manual_hash_new = {
            let mut hasher = light_poseidon::Poseidon::<Fr>::new_circom(7).unwrap();
            use ark_bn254::Fr;
            use ark_ff::{BigInteger, PrimeField};
            let hashed_owner = hash_to_bn254_field_size_be(&owner.to_bytes());
            let owner = Fr::from_be_bytes_mod_order(hashed_owner.as_slice());
            let leaf_index = Fr::from_be_bytes_mod_order(leaf_index.to_be_bytes().as_ref());
            let hashed_mt = hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes());
            let merkle_tree_pubkey = Fr::from_be_bytes_mod_order(hashed_mt.as_slice());
            let lamports = Fr::from_be_bytes_mod_order(lamports.to_be_bytes().as_ref())
                + Fr::from_be_bytes_mod_order(&[1u8, 0, 0, 0, 0, 0, 0, 0, 0]);
            let address = Fr::from_be_bytes_mod_order(address.as_slice());
            let discriminator = Fr::from_be_bytes_mod_order(data.discriminator.as_ref());
            let domain_separated_discriminator =
                Fr::from_be_bytes_mod_order(&[2, 0, 0, 0, 0, 0, 0, 0, 0]);
            let data_discriminator = discriminator + domain_separated_discriminator;
            use light_poseidon::PoseidonHasher;
            let inputs = [
                owner,
                leaf_index,
                merkle_tree_pubkey,
                lamports,
                address,
                data_discriminator,
                Fr::from_be_bytes_mod_order(data.data_hash.as_ref()),
            ];
            hasher.hash(&inputs).unwrap().into_bigint().to_bytes_be()
        };
        let hash = compressed_account
            .hash(&merkle_tree_pubkey, &leaf_index, true)
            .unwrap();
        let z_hash = z_account
            .hash(&merkle_tree_pubkey.to_bytes(), &leaf_index, true)
            .unwrap();
        assert_ne!(hash.to_vec(), manual_hash);
        assert_eq!(hash.to_vec(), manual_hash_new);
        assert_eq!(z_hash.to_vec(), manual_hash_new);
        assert_eq!(hash.len(), 32);
        use std::str::FromStr;
        let circuit_reference_value = BigUint::from_str(
            "15638319165413000277907073391141043184436601830909724248083671155000605125280",
        )
        .unwrap()
        .to_bytes_be();
        println!(
            "lamports domain: {:?}",
            BigUint::from_bytes_be(&[1u8, 0, 0, 0, 0, 0, 0, 0, 0]).to_string()
        );
        println!(
            "discriminator domain: {:?}",
            BigUint::from_bytes_be(&[2u8, 0, 0, 0, 0, 0, 0, 0, 0]).to_string()
        );
        assert_eq!(hash.to_vec(), circuit_reference_value);
    }

    impl CompressedAccount {
        pub fn legacy_hash_with_values<H: Hasher>(
            &self,
            &owner_hashed: &[u8; 32],
            &merkle_tree_hashed: &[u8; 32],
            leaf_index: &u32,
        ) -> Result<[u8; 32], CompressedAccountError> {
            let capacity = 3
                + std::cmp::min(self.lamports, 1) as usize
                + self.address.is_some() as usize
                + self.data.is_some() as usize * 2;
            let mut vec: Vec<&[u8]> = Vec::with_capacity(capacity);
            vec.push(owner_hashed.as_slice());

            // leaf index and merkle tree pubkey are used to make every compressed account hash unique
            let leaf_index = leaf_index.to_le_bytes();
            vec.push(leaf_index.as_slice());

            vec.push(merkle_tree_hashed.as_slice());

            // Lamports are only hashed if non-zero to save CU.
            // For safety, we prefix lamports with 1 in 1 byte.
            // Thus, even if the discriminator has the same value as the lamports, the hash will be different.
            let mut lamports_bytes = [1, 0, 0, 0, 0, 0, 0, 0, 0];
            if self.lamports != 0 {
                lamports_bytes[1..].copy_from_slice(&self.lamports.to_le_bytes());
                vec.push(lamports_bytes.as_slice());
            }

            if self.address.is_some() {
                vec.push(self.address.as_ref().unwrap().as_slice());
            }

            let mut discriminator_bytes = [2, 0, 0, 0, 0, 0, 0, 0, 0];
            if let Some(data) = &self.data {
                discriminator_bytes[1..].copy_from_slice(&data.discriminator);
                vec.push(&discriminator_bytes);
                vec.push(&data.data_hash);
            }
            let hash = H::hashv(&vec)?;
            Ok(hash)
        }

        pub fn hash_legacy<H: Hasher>(
            &self,
            &merkle_tree_pubkey: &Pubkey,
            leaf_index: &u32,
        ) -> Result<[u8; 32], CompressedAccountError> {
            let hashed_mt = hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes());
            self.legacy_hash_with_values::<H>(
                &hash_to_bn254_field_size_be(&self.owner.to_bytes()),
                &hashed_mt,
                leaf_index,
            )
        }
    }

    fn equivalency_of_hash_functions_rnd_iters<const ITERS: usize>() {
        let mut rng = rand::thread_rng();

        for _ in 0..ITERS {
            let account = CompressedAccount {
                owner: Pubkey::new_unique(),
                lamports: rng.gen::<u64>(),
                address: if rng.gen_bool(0.5) {
                    let mut address = rng.gen::<[u8; 32]>();
                    address[0] = 0;
                    Some(address)
                } else {
                    None
                },
                data: if rng.gen_bool(0.5) {
                    Some(CompressedAccountData {
                        discriminator: rng.gen(),
                        data: Vec::new(), // not used in hash
                        data_hash: Poseidon::hash(rng.gen::<u64>().to_be_bytes().as_slice())
                            .unwrap(),
                    })
                } else {
                    None
                },
            };
            let leaf_index = rng.gen::<u32>();
            let merkle_tree_pubkey = Pubkey::new_unique();
            let hash_legacy = account
                .hash_legacy::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
                .unwrap();
            let hash = account
                .hash(&merkle_tree_pubkey, &leaf_index, false)
                .unwrap();
            let bytes: Vec<u8> = account.try_to_vec().unwrap();
            let (z_account, _) = ZCompressedAccount::zero_copy_at(bytes.as_slice()).unwrap();
            let z_hash = z_account
                .hash(&merkle_tree_pubkey.to_bytes(), &leaf_index, false)
                .unwrap();
            assert_eq!(hash_legacy, hash);
            assert_eq!(hash, z_hash);
        }
    }

    #[test]
    fn equivalency_of_hash_functions() {
        equivalency_of_hash_functions_rnd_iters::<100>();
    }
}
