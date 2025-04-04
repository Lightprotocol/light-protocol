use std::collections::HashMap;

use crate::pubkey::Pubkey;
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_zero_copy::{ZeroCopy, ZeroCopyEq};

use crate::{
    compressed_account::{CompressedAccount, PackedCompressedAccountWithMerkleContext},
    instruction_data::compressed_proof::CompressedProof,
};

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize, ZeroCopy)]
pub struct InstructionDataInvoke {
    pub proof: Option<CompressedProof>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct OutputCompressedAccountWithContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree: Pubkey,
}

#[derive(
    Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize, ZeroCopy, ZeroCopyEq,
)]
pub struct OutputCompressedAccountWithPackedContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree_index: u8,
}

#[derive(
    Debug, PartialEq, Default, Clone, Copy, AnchorDeserialize, AnchorSerialize, ZeroCopy, ZeroCopyEq,
)]
pub struct NewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct NewAddressParams {
    pub seed: [u8; 32],
    pub address_queue_pubkey: solana_program::pubkey::Pubkey,
    pub address_merkle_tree_pubkey: solana_program::pubkey::Pubkey,
    pub address_merkle_tree_root_index: u16,
}

#[derive(
    Debug, PartialEq, Default, Clone, Copy, AnchorDeserialize, AnchorSerialize, ZeroCopy, ZeroCopyEq,
)]
pub struct PackedReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_root_index: u16,
    pub address_merkle_tree_account_index: u8,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct ReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_pubkey: solana_program::pubkey::Pubkey,
    pub address_merkle_tree_root_index: u16,
}

pub fn pack_pubkey(
    pubkey: &solana_program::pubkey::Pubkey,
    hash_set: &mut HashMap<solana_program::pubkey::Pubkey, u8>,
) -> u8 {
    match hash_set.get(pubkey) {
        Some(index) => *index,
        None => {
            let index = hash_set.len() as u8;
            hash_set.insert(*pubkey, index);
            index
        }
    }
}
// TODO: add randomized tests
// TODO: add unit test ZInstructionDataInvokeCpiWithReadOnly
#[cfg(test)]
mod test {
    use borsh::BorshSerialize;
    use light_zero_copy::borsh::Deserialize;
    use rand::{
        rngs::{StdRng, ThreadRng},
        Rng,
    };

    use super::*;
    use crate::{
        compressed_account::{
            CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
            PackedMerkleContext, ZCompressedAccount, ZCompressedAccountData,
            ZPackedCompressedAccountWithMerkleContext, ZPackedMerkleContext,
        },
        instruction_data::{
            cpi_context::CompressedCpiContext,
            data::{InstructionDataInvoke, NewAddressParamsPacked},
            invoke_cpi::{InstructionDataInvokeCpi, ZInstructionDataInvokeCpi},
        },
        CompressedAccountError,
    };

    fn get_instruction_data_invoke_cpi() -> InstructionDataInvokeCpi {
        InstructionDataInvokeCpi {
            proof: Some(CompressedProof {
                a: [1; 32],
                b: [2; 64],
                c: [3; 32],
            }),
            new_address_params: vec![get_new_address_params(); 3],
            input_compressed_accounts_with_merkle_context: vec![get_test_input_account(); 3],
            output_compressed_accounts: vec![get_test_output_account(); 2],
            relay_fee: None,
            compress_or_decompress_lamports: Some(1),
            is_compress: true,
            cpi_context: Some(get_cpi_context()),
        }
    }

    fn get_rnd_instruction_data_invoke_cpi(rng: &mut StdRng) -> InstructionDataInvokeCpi {
        InstructionDataInvokeCpi {
            proof: Some(CompressedProof {
                a: rng.gen(),
                b: (0..64)
                    .map(|_| rng.gen())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .unwrap(),
                c: rng.gen(),
            }),
            new_address_params: vec![get_rnd_new_address_params(rng); rng.gen_range(0..10)],
            input_compressed_accounts_with_merkle_context: vec![
                get_rnd_test_input_account(rng);
                rng.gen_range(0..10)
            ],
            output_compressed_accounts: vec![
                get_rnd_test_output_account(rng);
                rng.gen_range(0..10)
            ],
            relay_fee: None,
            compress_or_decompress_lamports: rng.gen(),
            is_compress: rng.gen(),
            cpi_context: Some(get_rnd_cpi_context(rng)),
        }
    }

    fn compare_invoke_cpi_instruction_data(
        reference: &InstructionDataInvokeCpi,
        z_copy: &ZInstructionDataInvokeCpi,
    ) -> Result<(), CompressedAccountError> {
        if reference.proof.is_some() && z_copy.proof.is_none() {
            println!("proof is none");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.proof.is_none() && z_copy.proof.is_some() {
            println!("proof is some");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.proof.is_some()
            && z_copy.proof.is_some()
            && reference.proof.as_ref().unwrap().a != z_copy.proof.as_ref().unwrap().a
            || reference.proof.as_ref().unwrap().b != z_copy.proof.as_ref().unwrap().b
            || reference.proof.as_ref().unwrap().c != z_copy.proof.as_ref().unwrap().c
        {
            println!("proof is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference
            .input_compressed_accounts_with_merkle_context
            .len()
            != z_copy.input_compressed_accounts_with_merkle_context.len()
        {
            println!("input_compressed_accounts_with_merkle_context is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_input, z_input) in reference
            .input_compressed_accounts_with_merkle_context
            .iter()
            .zip(z_copy.input_compressed_accounts_with_merkle_context.iter())
        {
            compare_packed_compressed_account_with_merkle_context(ref_input, z_input)?;
        }
        if reference.output_compressed_accounts.len() != z_copy.output_compressed_accounts.len() {
            println!("output_compressed_accounts is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_output, z_output) in reference
            .output_compressed_accounts
            .iter()
            .zip(z_copy.output_compressed_accounts.iter())
        {
            compare_compressed_output_account(ref_output, z_output)?;
        }
        if reference.relay_fee != z_copy.relay_fee.map(|x| (*x).into()) {
            println!("relay_fee is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.new_address_params.len() != z_copy.new_address_params.len() {
            println!("new_address_params is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_params, z_params) in reference
            .new_address_params
            .iter()
            .zip(z_copy.new_address_params.iter())
        {
            if ref_params.seed != z_params.seed {
                println!("seed is not equal");
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_queue_account_index != z_params.address_queue_account_index {
                println!("address_queue_account_index is not equal");
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_merkle_tree_account_index
                != z_params.address_merkle_tree_account_index
            {
                println!("address_merkle_tree_account_index is not equal");
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_merkle_tree_root_index
                != u16::from(z_params.address_merkle_tree_root_index)
            {
                println!("address_merkle_tree_root_index is not equal");
                return Err(CompressedAccountError::InvalidArgument);
            }
        }
        if reference.compress_or_decompress_lamports
            != z_copy.compress_or_decompress_lamports.map(|x| (*x).into())
        {
            println!("compress_or_decompress_lamports is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.is_compress != z_copy.is_compress() {
            println!("is_compress is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.cpi_context.is_some() && z_copy.cpi_context.is_none() {
            println!("cpi_context is none");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.cpi_context.is_none() && z_copy.cpi_context.is_some() {
            println!("cpi_context is some");
            println!("reference: {:?}", reference.cpi_context);
            println!("z_copy: {:?}", z_copy.cpi_context);
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.cpi_context.is_some() && z_copy.cpi_context.is_some() {
            let reference = reference.cpi_context.as_ref().unwrap();
            let zcopy = z_copy.cpi_context.as_ref().unwrap();
            if reference.first_set_context != zcopy.first_set_context()
                || reference.set_context != zcopy.set_context()
                || reference.cpi_context_account_index != zcopy.cpi_context_account_index
            {
                println!("reference: {:?}", reference);
                println!("z_copy: {:?}", zcopy);
                return Err(CompressedAccountError::InvalidArgument);
            }
        }
        Ok(())
    }

    #[test]
    fn test_cpi_context_instruction_data() {
        let reference = get_instruction_data_invoke_cpi();

        let mut bytes = Vec::new();
        reference.serialize(&mut bytes).unwrap();
        let (z_copy, bytes) = InstructionDataInvokeCpi::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        compare_invoke_cpi_instruction_data(&reference, &z_copy).unwrap();
    }

    fn get_cpi_context() -> CompressedCpiContext {
        CompressedCpiContext {
            first_set_context: true,
            set_context: true,
            cpi_context_account_index: 1,
        }
    }

    fn get_rnd_cpi_context(rng: &mut StdRng) -> CompressedCpiContext {
        CompressedCpiContext {
            first_set_context: rng.gen(),
            set_context: rng.gen(),
            cpi_context_account_index: rng.gen(),
        }
    }

    #[test]
    fn test_cpi_context_deserialize() {
        let cpi_context = get_cpi_context();
        let mut bytes = Vec::new();
        cpi_context.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = CompressedCpiContext::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        assert_eq!(z_copy, cpi_context);
    }

    #[test]
    fn test_account_deserialize() {
        let test_account = get_test_account();
        let mut bytes = Vec::new();
        test_account.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = CompressedAccount::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        compare_compressed_account(&test_account, &z_copy).unwrap();
    }

    fn get_test_account_data() -> CompressedAccountData {
        CompressedAccountData {
            discriminator: 1u64.to_le_bytes(),
            data: vec![1, 2, 3, 4, 5, 6, 7, 8],
            data_hash: [1; 32],
        }
    }

    fn get_rnd_test_account_data(rng: &mut StdRng) -> CompressedAccountData {
        CompressedAccountData {
            discriminator: rng.gen(),
            data: (0..100).map(|_| rng.gen()).collect::<Vec<u8>>(),
            data_hash: rng.gen(),
        }
    }

    fn get_test_account() -> CompressedAccount {
        CompressedAccount {
            owner: crate::pubkey::Pubkey::new_unique(),
            lamports: 100,
            address: Some(Pubkey::new_unique().to_bytes()),
            data: Some(get_test_account_data()),
        }
    }

    fn get_rnd_test_account(rng: &mut StdRng) -> CompressedAccount {
        CompressedAccount {
            owner: crate::pubkey::Pubkey::new_unique(),
            lamports: rng.gen(),
            address: Some(Pubkey::new_unique().to_bytes()),
            data: Some(get_rnd_test_account_data(rng)),
        }
    }

    fn get_test_output_account() -> OutputCompressedAccountWithPackedContext {
        OutputCompressedAccountWithPackedContext {
            compressed_account: get_test_account(),
            merkle_tree_index: 1,
        }
    }

    fn get_rnd_test_output_account(rng: &mut StdRng) -> OutputCompressedAccountWithPackedContext {
        OutputCompressedAccountWithPackedContext {
            compressed_account: get_rnd_test_account(rng),
            merkle_tree_index: rng.gen(),
        }
    }

    #[test]
    fn test_output_account_deserialize() {
        let test_output_account = get_test_output_account();
        let mut bytes = Vec::new();
        test_output_account.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) =
            OutputCompressedAccountWithPackedContext::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        compare_compressed_output_account(&test_output_account, &z_copy).unwrap();
    }

    fn compare_compressed_output_account(
        reference: &OutputCompressedAccountWithPackedContext,
        z_copy: &ZOutputCompressedAccountWithPackedContext,
    ) -> Result<(), CompressedAccountError> {
        compare_compressed_account(&reference.compressed_account, &z_copy.compressed_account)?;
        if reference.merkle_tree_index != z_copy.merkle_tree_index {
            return Err(CompressedAccountError::InvalidArgument);
        }
        Ok(())
    }

    fn get_test_input_account() -> PackedCompressedAccountWithMerkleContext {
        PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: crate::pubkey::Pubkey::new_unique(),
                lamports: 100,
                address: Some(Pubkey::new_unique().to_bytes()),
                data: Some(CompressedAccountData {
                    discriminator: 1u64.to_le_bytes(),
                    data: vec![1, 2, 3, 4, 5, 6, 7, 8],
                    data_hash: [1; 32],
                }),
            },
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                nullifier_queue_pubkey_index: 2,
                leaf_index: 3,
                prove_by_index: true,
            },
            root_index: 5,
            read_only: false,
        }
    }

    fn get_rnd_test_input_account(rng: &mut StdRng) -> PackedCompressedAccountWithMerkleContext {
        PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: crate::pubkey::Pubkey::new_unique(),
                lamports: 100,
                address: Some(Pubkey::new_unique().to_bytes()),
                data: Some(get_rnd_test_account_data(rng)),
            },
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: rng.gen(),
                nullifier_queue_pubkey_index: rng.gen(),
                leaf_index: rng.gen(),
                prove_by_index: rng.gen(),
            },
            root_index: rng.gen(),
            read_only: false,
        }
    }
    #[test]
    fn test_input_account_deserialize() {
        let input_account = get_test_input_account();

        let mut bytes = Vec::new();
        input_account.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) =
            PackedCompressedAccountWithMerkleContext::zero_copy_at(&bytes).unwrap();

        assert!(bytes.is_empty());
        compare_packed_compressed_account_with_merkle_context(&input_account, &z_copy).unwrap();
    }

    fn get_new_address_params() -> NewAddressParamsPacked {
        NewAddressParamsPacked {
            seed: [1; 32],
            address_queue_account_index: 1,
            address_merkle_tree_account_index: 2,
            address_merkle_tree_root_index: 3,
        }
    }

    // get_instruction_data_invoke_cpi
    fn get_rnd_new_address_params(rng: &mut StdRng) -> NewAddressParamsPacked {
        NewAddressParamsPacked {
            seed: rng.gen(),
            address_queue_account_index: rng.gen(),
            address_merkle_tree_account_index: rng.gen(),
            address_merkle_tree_root_index: rng.gen(),
        }
    }
    #[test]
    fn test_account_data_deserialize() {
        let test_data = CompressedAccountData {
            discriminator: 1u64.to_le_bytes(),
            data: vec![1, 2, 3, 4, 5, 6, 7, 8],
            data_hash: [1; 32],
        };

        let mut bytes = Vec::new();
        test_data.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = CompressedAccountData::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        assert_eq!(
            z_copy.discriminator.as_slice(),
            test_data.discriminator.as_slice()
        );
        assert_eq!(z_copy.data, test_data.data.as_slice());
        assert_eq!(z_copy.data_hash.as_slice(), test_data.data_hash.as_slice());
    }

    #[test]
    fn test_invoke_ix_data_deserialize() {
        let invoke_ref = InstructionDataInvoke {
            proof: Some(CompressedProof {
                a: [1; 32],
                b: [2; 64],
                c: [3; 32],
            }),
            input_compressed_accounts_with_merkle_context: vec![get_test_input_account(); 2],
            output_compressed_accounts: vec![get_test_output_account(); 2],
            relay_fee: None,
            new_address_params: vec![get_new_address_params(); 2],
            compress_or_decompress_lamports: Some(1),
            is_compress: true,
        };
        let mut bytes = Vec::new();
        invoke_ref.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = InstructionDataInvoke::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        compare_instruction_data(&invoke_ref, &z_copy).unwrap();
    }

    fn compare_instruction_data(
        reference: &InstructionDataInvoke,
        z_copy: &ZInstructionDataInvoke,
    ) -> Result<(), CompressedAccountError> {
        if reference.proof.is_some() && z_copy.proof.is_none() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.proof.is_none() && z_copy.proof.is_some() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.proof.is_some()
            && z_copy.proof.is_some()
            && reference.proof.as_ref().unwrap().a != z_copy.proof.as_ref().unwrap().a
            || reference.proof.as_ref().unwrap().b != z_copy.proof.as_ref().unwrap().b
            || reference.proof.as_ref().unwrap().c != z_copy.proof.as_ref().unwrap().c
        {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference
            .input_compressed_accounts_with_merkle_context
            .len()
            != z_copy.input_compressed_accounts_with_merkle_context.len()
        {
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_input, z_input) in reference
            .input_compressed_accounts_with_merkle_context
            .iter()
            .zip(z_copy.input_compressed_accounts_with_merkle_context.iter())
        {
            compare_packed_compressed_account_with_merkle_context(ref_input, z_input)?;
        }
        if reference.output_compressed_accounts.len() != z_copy.output_compressed_accounts.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_output, z_output) in reference
            .output_compressed_accounts
            .iter()
            .zip(z_copy.output_compressed_accounts.iter())
        {
            compare_compressed_output_account(ref_output, z_output)?;
        }
        if reference.relay_fee != z_copy.relay_fee.map(|x| (*x).into()) {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.new_address_params.len() != z_copy.new_address_params.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_params, z_params) in reference
            .new_address_params
            .iter()
            .zip(z_copy.new_address_params.iter())
        {
            if ref_params.seed != z_params.seed {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_queue_account_index != z_params.address_queue_account_index {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_merkle_tree_account_index
                != z_params.address_merkle_tree_account_index
            {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_merkle_tree_root_index
                != u16::from(z_params.address_merkle_tree_root_index)
            {
                return Err(CompressedAccountError::InvalidArgument);
            }
        }
        Ok(())
    }

    fn compare_compressed_account_data(
        reference: &CompressedAccountData,
        z_copy: &ZCompressedAccountData,
    ) -> Result<(), CompressedAccountError> {
        if reference.discriminator.as_slice() != z_copy.discriminator.as_slice() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.data != z_copy.data {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.data_hash.as_slice() != z_copy.data_hash.as_slice() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        Ok(())
    }

    fn compare_compressed_account(
        reference: &CompressedAccount,
        z_copy: &ZCompressedAccount,
    ) -> Result<(), CompressedAccountError> {
        if reference.owner != z_copy.owner {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.lamports != u64::from(z_copy.lamports) {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.address != z_copy.address.map(|x| *x) {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.data.is_some() && z_copy.data.is_none() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.data.is_none() && z_copy.data.is_some() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.data.is_some() && z_copy.data.is_some() {
            compare_compressed_account_data(
                reference.data.as_ref().unwrap(),
                z_copy.data.as_ref().unwrap(),
            )?;
        }
        Ok(())
    }

    fn compare_merkle_context(
        reference: PackedMerkleContext,
        z_copy: &ZPackedMerkleContext,
    ) -> Result<(), CompressedAccountError> {
        if reference.merkle_tree_pubkey_index != z_copy.merkle_tree_pubkey_index {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.nullifier_queue_pubkey_index != z_copy.nullifier_queue_pubkey_index {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.leaf_index != u32::from(z_copy.leaf_index) {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.prove_by_index != (z_copy.prove_by_index == 1) {
            return Err(CompressedAccountError::InvalidArgument);
        }
        Ok(())
    }

    fn compare_packed_compressed_account_with_merkle_context(
        reference: &PackedCompressedAccountWithMerkleContext,
        z_copy: &ZPackedCompressedAccountWithMerkleContext,
    ) -> Result<(), CompressedAccountError> {
        compare_compressed_account(&reference.compressed_account, &z_copy.compressed_account)?;
        compare_merkle_context(reference.merkle_context, &z_copy.merkle_context)?;
        if reference.root_index != u16::from(*z_copy.root_index) {
            return Err(CompressedAccountError::InvalidArgument);
        }

        Ok(())
    }

    #[test]
    fn test_instruction_data_invoke_cpi_rnd() {
        use rand::{rngs::StdRng, Rng, SeedableRng};
        let mut thread_rng = ThreadRng::default();
        let seed = thread_rng.gen();
        // Keep this print so that in case the test fails
        // we can use the seed to reproduce the error.
        println!("\n\ne2e test seed {}\n\n", seed);
        let mut rng = StdRng::seed_from_u64(seed);

        let num_iters = 10000;
        for _ in 0..num_iters {
            let value = get_rnd_instruction_data_invoke_cpi(&mut rng);
            let mut vec = Vec::new();
            value.serialize(&mut vec).unwrap();
            let (zero_copy, _) = InstructionDataInvokeCpi::zero_copy_at(&vec).unwrap();
            compare_invoke_cpi_instruction_data(&value, &zero_copy).unwrap();
        }
    }
}
