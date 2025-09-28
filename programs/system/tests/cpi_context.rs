//! Set cpi context tests:
//! 1. Functional: Set cpi context first invocation
//! 2. Functional: Set cpi context subsequent invocation
//! 3. Failing: Set cpi context fee payer mismatch
//! 4. Failing: Set cpi context without first context
//!
//! process cpi context:
//! 1. CpiContextMissing
//! 2. CpiContextAccountUndefined
//! 3. NoInputs
//! 4. CpiContextAssociatedMerkleTreeMismatch
//! 5. CpiContextEmpty
//! 6. CpiContextFeePayerMismatch
//!
//! Functional process cpi context:
//! 1. Set context
//! 2. Combine (with malicious input in cpi context account)

use borsh::BorshSerialize;
use light_account_checks::account_info::test_account_info::pinocchio::get_account_info;
#[cfg(test)]
use light_compressed_account::instruction_data::traits::InstructionData;
use light_compressed_account::{
    compressed_account::{
        CompressedAccount, PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
    },
    instruction_data::{
        cpi_context::CompressedCpiContext,
        data::OutputCompressedAccountWithPackedContext,
        invoke_cpi::InstructionDataInvokeCpi,
        traits::{InputAccount, NewAddress, OutputAccount},
        zero_copy::{
            ZInstructionDataInvokeCpi, ZPackedMerkleContext, ZPackedReadOnlyAddress,
            ZPackedReadOnlyCompressedAccount,
        },
    },
};
use light_system_program_pinocchio::{
    context::WrappedInstructionData,
    cpi_context::{
        account::{CpiContextInAccount, CpiContextOutAccount},
        address::CpiContextNewAddressParamsAssignedPacked,
        process_cpi_context::{process_cpi_context, set_cpi_context},
        state::{
            cpi_context_account_new, deserialize_cpi_context_account,
            deserialize_cpi_context_account_cleared, CpiContextAccountInitParams,
        },
    },
    errors::SystemProgramError,
    ID,
};
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use rand::{rngs::StdRng, Rng, SeedableRng};
use zerocopy::little_endian::{U16, U32, U64};

/// Compares:
/// 1. new address
/// 2. input account
/// 3. output account
/// 4. read-only address
/// 5. read-only account
/// - other data is not compared
#[cfg(test)]
pub fn instruction_data_eq<'a>(
    left: &impl InstructionData<'a>,
    right: &impl InstructionData<'a>,
) -> bool {
    // Compare collections using our helper functions
    new_addresses_eq(left.new_addresses(), right.new_addresses()) &&
    input_accounts_eq(left.input_accounts(), right.input_accounts()) &&
    output_accounts_eq(left.output_accounts(), right.output_accounts()) &&
    // Compare read-only data
    left.read_only_addresses() == right.read_only_addresses() &&
    left.read_only_accounts() == right.read_only_accounts()
}

pub fn input_accounts_eq<'a>(
    left: &[impl InputAccount<'a>],
    right: &[impl InputAccount<'a>],
) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter().zip(right.iter()).all(|(l, r)| {
        l.owner() == r.owner()
            && l.lamports() == r.lamports()
            && l.address() == r.address()
            && l.merkle_context() == r.merkle_context()
            && l.skip() == r.skip()
            && l.has_data() == r.has_data()
            && l.data() == r.data()
            && l.root_index() == r.root_index()
    })
}

pub fn new_addresses_eq<'a>(left: &[impl NewAddress<'a>], right: &[impl NewAddress<'a>]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter().zip(right.iter()).all(|(l, r)| {
        l.seed() == r.seed()
            && l.address_queue_index() == r.address_queue_index()
            && l.address_merkle_tree_account_index() == r.address_merkle_tree_account_index()
            && l.address_merkle_tree_root_index() == r.address_merkle_tree_root_index()
            && l.assigned_compressed_account_index() == r.assigned_compressed_account_index()
    })
}

pub fn output_accounts_eq<'a>(
    left: &[impl OutputAccount<'a>],
    right: &[impl OutputAccount<'a>],
) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter().zip(right.iter()).all(|(l, r)| {
        l.owner() == r.owner()
            && l.lamports() == r.lamports()
            && l.address() == r.address()
            && l.merkle_tree_index() == r.merkle_tree_index()
            && l.skip() == r.skip()
            && l.has_data() == r.has_data()
            && l.data() == r.data()
    })
}

/// Calculate vector offsets for zero-copy vectors
/// Returns (length_offset, capacity_offset, data_start, data_end)
#[inline]
fn calculate_vector_offsets(
    data: &[u8],
    start: usize,
    element_size: usize,
) -> (usize, usize, usize, usize) {
    let length_offset = start;
    let capacity_offset = start + 1;
    let data_start = start + 2;
    let capacity = data[capacity_offset];
    let data_end = data_start + (capacity as usize * element_size);
    (length_offset, capacity_offset, data_start, data_end)
}

/// Assert that CPI context account bytes are properly cleared
/// Checks that all bytes are zero except:
/// - Discriminator (bytes 0-8)
/// - Associated merkle tree (bytes 40-72)
/// - Vector capacity fields (1 byte per vector at specific offsets)
fn assert_cpi_context_cleared_bytes(account_info: &AccountInfo, expected_merkle_tree: Pubkey) {
    let data = account_info.try_borrow_data().unwrap();

    // Define exact byte ranges for each field in the zero-copy structure
    let discriminator_start = 0;
    let discriminator_end = 8;

    let fee_payer_start = 8;
    let fee_payer_end = 40;

    let associated_merkle_tree_start = 40;
    let associated_merkle_tree_end = 72;
    let place_holder_data_end = 136;

    // Vector metadata: each has [length: 1 byte, capacity: 1 byte, data: capacity * element_size]
    // Element sizes using size_of for accuracy
    use light_compressed_account::instruction_data::zero_copy::{
        ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount,
    };

    let new_addresses_element_size =
        std::mem::size_of::<CpiContextNewAddressParamsAssignedPacked>();
    let readonly_addresses_element_size = std::mem::size_of::<ZPackedReadOnlyAddress>();
    let readonly_accounts_element_size = std::mem::size_of::<ZPackedReadOnlyCompressedAccount>();
    let in_accounts_element_size = std::mem::size_of::<CpiContextInAccount>();
    let out_accounts_element_size = std::mem::size_of::<CpiContextOutAccount>();

    // new_addresses vector
    // Vector layout is [length: 1 byte, capacity: 1 byte, data...]
    let (
        new_addresses_length_offset,
        _new_addresses_capacity_offset,
        new_addresses_data_start,
        new_addresses_data_end,
    ) = calculate_vector_offsets(&data, place_holder_data_end, new_addresses_element_size);

    // readonly_addresses vector
    let (
        readonly_addresses_length_offset,
        _readonly_addresses_capacity_offset,
        readonly_addresses_data_start,
        readonly_addresses_data_end,
    ) = calculate_vector_offsets(
        &data,
        new_addresses_data_end,
        readonly_addresses_element_size,
    );

    // readonly_accounts vector
    let (
        readonly_accounts_length_offset,
        _readonly_accounts_capacity_offset,
        readonly_accounts_data_start,
        readonly_accounts_data_end,
    ) = calculate_vector_offsets(
        &data,
        readonly_addresses_data_end,
        readonly_accounts_element_size,
    );

    // in_accounts vector
    let (
        in_accounts_length_offset,
        _in_accounts_capacity_offset,
        in_accounts_data_start,
        in_accounts_data_end,
    ) = calculate_vector_offsets(&data, readonly_accounts_data_end, in_accounts_element_size);

    // out_accounts vector
    let (
        out_accounts_length_offset,
        _out_accounts_capacity_offset,
        out_accounts_data_start,
        out_accounts_data_end,
    ) = calculate_vector_offsets(&data, in_accounts_data_end, out_accounts_element_size);

    // output_data_len (U16 - 2 bytes)
    let output_data_len_start = out_accounts_data_end;
    let output_data_len_end = out_accounts_data_end + 2;

    // Remaining bytes for output data
    let output_data_start = output_data_len_end;
    let output_data_end = data.len();

    // Now perform assertions on each field using slices

    // 1. Check discriminator (should match expected)
    assert_eq!(
        &data[discriminator_start..discriminator_end],
        &light_system_program_pinocchio::CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR
    );

    // 2. Check fee payer (should be all zeros)
    let fee_payer_slice = &data[fee_payer_start..fee_payer_end];
    assert!(
        !fee_payer_slice.iter().any(|&b| b != 0),
        "Fee payer should be all zeros but contains non-zero bytes"
    );

    // 3. Check associated merkle tree (should match expected)
    assert_eq!(
        &data[associated_merkle_tree_start..associated_merkle_tree_end],
        expected_merkle_tree.as_ref()
    );

    // 4. Check new_addresses vector
    assert_eq!(
        data[new_addresses_length_offset], 0,
        "new_addresses length should be 0"
    );
    let new_addresses_data = &data[new_addresses_data_start..new_addresses_data_end];
    assert!(
        !new_addresses_data.iter().any(|&b| b != 0),
        "new_addresses data should be all zeros"
    );

    // 5. Check readonly_addresses vector
    assert_eq!(
        data[readonly_addresses_length_offset], 0,
        "readonly_addresses length should be 0"
    );
    let readonly_addresses_data = &data[readonly_addresses_data_start..readonly_addresses_data_end];
    assert!(
        !readonly_addresses_data.iter().any(|&b| b != 0),
        "readonly_addresses data should be all zeros"
    );

    // 6. Check readonly_accounts vector
    assert_eq!(
        data[readonly_accounts_length_offset], 0,
        "readonly_accounts length should be 0"
    );
    let readonly_accounts_data = &data[readonly_accounts_data_start..readonly_accounts_data_end];
    assert!(
        !readonly_accounts_data.iter().any(|&b| b != 0),
        "readonly_accounts data should be all zeros"
    );

    // 7. Check in_accounts vector
    assert_eq!(
        data[in_accounts_length_offset], 0,
        "in_accounts length should be 0"
    );
    let in_accounts_data = &data[in_accounts_data_start..in_accounts_data_end];
    assert!(
        !in_accounts_data.iter().any(|&b| b != 0),
        "in_accounts data should be all zeros"
    );

    // 8. Check out_accounts vector
    assert_eq!(
        data[out_accounts_length_offset], 0,
        "out_accounts length should be 0"
    );
    let out_accounts_data = &data[out_accounts_data_start..out_accounts_data_end];
    assert!(
        !out_accounts_data.iter().any(|&b| b != 0),
        "out_accounts data should be all zeros"
    );

    // 9. Check output_data_len (should be 0)
    let output_data_len_slice = &data[output_data_len_start..output_data_len_end];
    assert!(
        !output_data_len_slice.iter().any(|&b| b != 0),
        "output_data_len should be all zeros"
    );

    // 10. Check remaining output data area (should all be zeros)
    let output_data = &data[output_data_start..output_data_end];
    assert!(
        !output_data.iter().any(|&b| b != 0),
        "output_data area should be all zeros"
    );
}

fn clean_input_data(instruction_data: &mut InstructionDataInvokeCpi) {
    instruction_data.cpi_context = None;
    instruction_data.compress_or_decompress_lamports = None;
    instruction_data.relay_fee = None;
    instruction_data.proof = None;
}

fn create_test_cpi_context_account(associated_merkle_tree: Option<Pubkey>) -> AccountInfo {
    let associated_merkle_tree =
        associated_merkle_tree.unwrap_or(solana_pubkey::Pubkey::new_unique().to_bytes());
    let params = CpiContextAccountInitParams::new(associated_merkle_tree);
    let account_info = get_account_info(
        solana_pubkey::Pubkey::new_unique().to_bytes(),
        crate::ID,
        false,
        true,
        false,
        vec![0u8; 20000],
    );
    cpi_context_account_new::<false>(&account_info, params).unwrap();
    account_info
}

fn create_test_instruction_data(
    first_set_context: bool,
    set_context: bool,
    iter: u8,
) -> InstructionDataInvokeCpi {
    InstructionDataInvokeCpi {
        proof: None,
        new_address_params: vec![],
        input_compressed_accounts_with_merkle_context: vec![
            PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: solana_pubkey::Pubkey::new_unique().to_bytes().into(),
                    lamports: iter.into(),
                    address: None,
                    data: None,
                },
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: iter,
                    leaf_index: 0,
                    prove_by_index: false,
                },
                root_index: iter.into(),
                read_only: false,
            },
        ],
        output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: solana_pubkey::Pubkey::new_unique().to_bytes().into(),
                lamports: iter.into(),
                address: None,
                data: None,
            },
            merkle_tree_index: iter,
        }],
        relay_fee: None,
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context: Some(CompressedCpiContext {
            first_set_context,
            set_context,
            cpi_context_account_index: 0,
        }),
    }
}

fn get_invalid_merkle_tree_account_info() -> AccountInfo {
    let data = vec![172, 43, 172, 186, 29, 73, 219, 84];
    get_account_info(
        solana_pubkey::Pubkey::new_unique().to_bytes(),
        crate::ID,
        false,
        true,
        false,
        data,
    )
}

fn get_merkle_tree_account_info() -> AccountInfo {
    let data = vec![22, 20, 149, 218, 74, 204, 128, 166];
    get_account_info(
        solana_pubkey::Pubkey::new_unique().to_bytes(),
        crate::ID,
        false,
        true,
        false,
        data,
    )
}

#[test]
fn test_set_cpi_context_first_invocation() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let cpi_context_account = create_test_cpi_context_account(None);

    let instruction_data = create_test_instruction_data(true, true, 1);
    let input_bytes = instruction_data.try_to_vec().unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = set_cpi_context(fee_payer, &cpi_context_account, w_instruction_data);
    // assert
    {
        assert!(result.is_ok());
        let input_bytes = instruction_data.try_to_vec().unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let cpi_context = deserialize_cpi_context_account(&cpi_context_account).unwrap();
        assert_eq!(cpi_context.fee_payer.to_bytes(), fee_payer);
        assert!(instruction_data_eq(&cpi_context, &z_inputs));
    }
}

#[test]
fn test_set_cpi_context_subsequent_invocation() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let cpi_context_account = create_test_cpi_context_account(None);
    let mut first_instruction_data = create_test_instruction_data(true, true, 1);
    // First invocation
    {
        let input_bytes = first_instruction_data.try_to_vec().unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        set_cpi_context(fee_payer, &cpi_context_account, w_instruction_data).unwrap();
    }
    let inputs_subsequent = create_test_instruction_data(false, true, 2);
    let mut input_bytes = Vec::new();
    inputs_subsequent.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = set_cpi_context(fee_payer, &cpi_context_account, w_instruction_data);
    // assert
    {
        assert!(result.is_ok());
        let input_bytes = inputs_subsequent.try_to_vec().unwrap();
        let (_z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let cpi_context = deserialize_cpi_context_account(&cpi_context_account).unwrap();
        assert_eq!(cpi_context.fee_payer.to_bytes(), fee_payer);

        // Create expected instruction data.
        clean_input_data(&mut first_instruction_data);
        first_instruction_data
            .output_compressed_accounts
            .extend(inputs_subsequent.output_compressed_accounts);
        first_instruction_data
            .input_compressed_accounts_with_merkle_context
            .extend(inputs_subsequent.input_compressed_accounts_with_merkle_context);

        let input_bytes = first_instruction_data.try_to_vec().unwrap();
        let (z_expected_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        // Assert that the CPI context contains the combined instruction data
        assert!(
            instruction_data_eq(&cpi_context, &z_expected_inputs),
            "CPI context should contain combined instruction data from both invocations"
        );
    }
}

#[test]
fn test_set_cpi_context_fee_payer_mismatch() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let cpi_context_account = create_test_cpi_context_account(None);
    let first_instruction_data = create_test_instruction_data(true, true, 1);
    // First invocation
    {
        let input_bytes = first_instruction_data.try_to_vec().unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        set_cpi_context(fee_payer, &cpi_context_account, w_instruction_data).unwrap();
    }

    let different_fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let inputs_subsequent = create_test_instruction_data(false, true, 2);
    let mut input_bytes = Vec::new();
    inputs_subsequent.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = set_cpi_context(
        different_fee_payer,
        &cpi_context_account,
        w_instruction_data,
    );
    assert_eq!(
        result.unwrap_err(),
        SystemProgramError::CpiContextFeePayerMismatch.into()
    );
}

#[test]
fn test_set_cpi_context_without_first_context() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let cpi_context_account = create_test_cpi_context_account(None);
    let inputs_first = create_test_instruction_data(false, true, 1);
    let mut input_bytes = Vec::new();
    inputs_first.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = set_cpi_context(fee_payer, &cpi_context_account, w_instruction_data);
    assert_eq!(
        result,
        Err(SystemProgramError::CpiContextFeePayerMismatch.into())
    );
}

/// Check: process cpi 1
#[test]
fn test_process_cpi_context_both_none() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let instruction_data = create_test_instruction_data(false, true, 1);
    let cpi_context_account: Option<&AccountInfo> = None;
    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();

    let result =
        process_cpi_context(w_instruction_data, cpi_context_account, fee_payer, &[]).unwrap_err();
    assert_eq!(
        result,
        SystemProgramError::CpiContextAccountUndefined.into()
    );
}

/// Check: process cpi 1
#[test]
fn test_process_cpi_context_account_none_context_some() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let instruction_data = create_test_instruction_data(false, true, 1);
    let cpi_context_account: Option<&AccountInfo> = None;
    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result =
        process_cpi_context(w_instruction_data, cpi_context_account, fee_payer, &[]).unwrap_err();
    assert_eq!(
        result,
        SystemProgramError::CpiContextAccountUndefined.into()
    );
}

/// Check: process cpi 2
#[test]
fn test_process_cpi_context_account_some_context_none() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let instruction_data = InstructionDataInvokeCpi {
        cpi_context: None,
        ..create_test_instruction_data(false, true, 1)
    };
    let cpi_context_account = create_test_cpi_context_account(None);

    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = process_cpi_context(
        w_instruction_data,
        Some(&cpi_context_account),
        fee_payer,
        &[],
    )
    .unwrap_err();
    assert_eq!(result, SystemProgramError::CpiContextMissing.into());
}

/// Check: process cpi 3
#[test]
fn test_process_cpi_no_inputs() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let mut instruction_data = create_test_instruction_data(false, false, 1);
    instruction_data.input_compressed_accounts_with_merkle_context = vec![];
    instruction_data.output_compressed_accounts = vec![];
    instruction_data.new_address_params = vec![];

    let merkle_tree_account_info = get_merkle_tree_account_info();
    let cpi_context_account =
        create_test_cpi_context_account(Some(*merkle_tree_account_info.key()));
    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = process_cpi_context(
        w_instruction_data,
        Some(&cpi_context_account),
        fee_payer,
        &[merkle_tree_account_info],
    )
    .unwrap_err();
    assert_eq!(result, SystemProgramError::NoInputs.into());
}

/// Check: process cpi 4
#[test]
fn test_process_cpi_context_associated_tree_mismatch() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let mut instruction_data = create_test_instruction_data(true, true, 1);
    instruction_data
        .cpi_context
        .as_mut()
        .unwrap()
        .first_set_context = false;
    instruction_data.cpi_context.as_mut().unwrap().set_context = false;
    let cpi_context_account = create_test_cpi_context_account(None);
    let merkle_tree_account_info = get_invalid_merkle_tree_account_info();
    let remaining_accounts = &[merkle_tree_account_info];
    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = process_cpi_context(
        w_instruction_data,
        Some(&cpi_context_account),
        fee_payer,
        remaining_accounts,
    )
    .unwrap_err();
    assert_eq!(
        result,
        SystemProgramError::CpiContextAssociatedMerkleTreeMismatch.into()
    );
}

/// Check: process cpi 5
#[test]
fn test_process_cpi_context_no_set_context() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let instruction_data = create_test_instruction_data(false, false, 1);
    let merkle_tree_account_info = get_merkle_tree_account_info();
    let cpi_context_account =
        create_test_cpi_context_account(Some(*merkle_tree_account_info.key()));
    let remaining_accounts = &[merkle_tree_account_info];
    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = process_cpi_context(
        w_instruction_data,
        Some(&cpi_context_account),
        fee_payer,
        remaining_accounts,
    )
    .unwrap_err();
    assert_eq!(result, SystemProgramError::CpiContextEmpty.into());
}

/// Check: process cpi 6
#[test]
fn test_process_cpi_context_empty_context_error() {
    let fee_payer = Pubkey::default();
    let instruction_data = create_test_instruction_data(false, true, 1);
    let merkle_tree_account_info = get_merkle_tree_account_info();
    let cpi_context_account =
        create_test_cpi_context_account(Some(*merkle_tree_account_info.key()));
    let remaining_accounts = &[merkle_tree_account_info];
    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = process_cpi_context(
        w_instruction_data,
        Some(&cpi_context_account),
        fee_payer,
        remaining_accounts,
    )
    .unwrap_err();
    assert_eq!(
        result,
        SystemProgramError::CpiContextFeePayerMismatch.into()
    );
}

/// Check: process cpi 6
#[test]
fn test_process_cpi_context_fee_payer_mismatch_error() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let instruction_data = create_test_instruction_data(true, true, 1);
    let merkle_tree_account_info = get_merkle_tree_account_info();
    let cpi_context_account =
        create_test_cpi_context_account(Some(*merkle_tree_account_info.key()));
    let remaining_accounts = &[merkle_tree_account_info];
    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = process_cpi_context(
        w_instruction_data,
        Some(&cpi_context_account),
        fee_payer,
        remaining_accounts,
    );
    assert!(result.is_ok());
    let invalid_fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let instruction_data = create_test_instruction_data(false, true, 1);
    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = process_cpi_context(
        w_instruction_data,
        Some(&cpi_context_account),
        invalid_fee_payer,
        remaining_accounts,
    )
    .unwrap_err();
    assert_eq!(
        result,
        SystemProgramError::CpiContextFeePayerMismatch.into()
    );
}

#[test]
fn test_process_cpi_context_set_context() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let mut instruction_data = create_test_instruction_data(true, true, 1);
    let merkle_tree_account_info = get_merkle_tree_account_info();
    let cpi_context_account =
        create_test_cpi_context_account(Some(*merkle_tree_account_info.key()));
    let remaining_accounts = &[merkle_tree_account_info];
    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = process_cpi_context(
        w_instruction_data,
        Some(&cpi_context_account),
        fee_payer,
        remaining_accounts,
    );
    // assert
    {
        assert!(result.is_ok());

        let cpi_context = deserialize_cpi_context_account(&cpi_context_account).unwrap();

        // Create expected instruction data.
        clean_input_data(&mut instruction_data);
        let input_bytes = instruction_data.try_to_vec().unwrap();
        let (z_expected_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        // Assert that the CPI context contains the instruction data
        assert!(
            instruction_data_eq(&cpi_context, &z_expected_inputs),
            "CPI context should contain the instruction data after first invocation"
        );
        assert!(result.unwrap().is_none());
    }
}

#[test]
fn test_process_cpi_context_scenario() {
    let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
    let mut instruction_data = create_test_instruction_data(true, true, 1);
    let malicious_inputs = create_test_instruction_data(true, true, 100);
    let merkle_tree_account_info = get_merkle_tree_account_info();
    let merkle_tree_pubkey = *merkle_tree_account_info.key();
    let cpi_context_account = create_test_cpi_context_account(Some(merkle_tree_pubkey));
    // Inject malicious data into cpi context account by setting context with malicious inputs.
    {
        // Set the malicious data as if it was the first invocation
        let input_bytes = malicious_inputs.try_to_vec().unwrap();
        let (z_malicious_inputs, _) =
            ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_malicious_instruction_data = WrappedInstructionData::new(z_malicious_inputs).unwrap();
        // Use set_cpi_context with Pubkey::default() as fee payer to inject the malicious data
        let mut cpi_context =
            deserialize_cpi_context_account_cleared(&cpi_context_account).unwrap();
        *cpi_context.fee_payer = Pubkey::default().into();
        cpi_context
            .store_data(&w_malicious_instruction_data)
            .unwrap();
    }

    let remaining_accounts = &[merkle_tree_account_info];
    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
    let result = process_cpi_context(
        w_instruction_data,
        Some(&cpi_context_account),
        fee_payer,
        remaining_accounts,
    );
    {
        assert!(result.is_ok());
        let cpi_context = deserialize_cpi_context_account(&cpi_context_account).unwrap();
        // Create expected instruction data.
        clean_input_data(&mut instruction_data);
        let input_bytes = instruction_data.try_to_vec().unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        assert!(instruction_data_eq(&cpi_context, &z_inputs));
        assert_eq!(
            cpi_context.associated_merkle_tree.to_bytes(),
            merkle_tree_pubkey
        );
        assert!(result.unwrap().is_none());
    }

    for i in 2..10 {
        let inputs_subsequent = create_test_instruction_data(false, true, i);
        let mut input_bytes = Vec::new();
        inputs_subsequent.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = process_cpi_context(
            w_instruction_data,
            Some(&cpi_context_account),
            fee_payer,
            remaining_accounts,
        );
        // assert
        {
            assert!(result.is_ok());
            let input_bytes = inputs_subsequent.try_to_vec().unwrap();
            let (z_inputs_subsequent, _) =
                ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
            let cpi_context = deserialize_cpi_context_account(&cpi_context_account).unwrap();
            assert_eq!(cpi_context.fee_payer.to_bytes(), fee_payer);
            // The context should not be empty after set_context
            assert!(!cpi_context.is_empty());
            // The context should NOT contain the current subsequent inputs (not combined yet)
            assert!(!instruction_data_eq(&cpi_context, &z_inputs_subsequent));
            instruction_data
                .output_compressed_accounts
                .extend(inputs_subsequent.output_compressed_accounts);
            instruction_data
                .input_compressed_accounts_with_merkle_context
                .extend(inputs_subsequent.input_compressed_accounts_with_merkle_context);

            let input_bytes = instruction_data.try_to_vec().unwrap();
            let (z_combined_inputs, _) =
                ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
            assert!(instruction_data_eq(&cpi_context, &z_combined_inputs));
        }
    }

    let instruction_data = create_test_instruction_data(false, false, 10);
    let mut input_bytes = Vec::new();
    instruction_data.serialize(&mut input_bytes).unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();

    let result = process_cpi_context(
        w_instruction_data,
        Some(&cpi_context_account),
        fee_payer,
        remaining_accounts,
    );
    assert!(result.is_ok());
    let (_, result) = result.unwrap().unwrap();

    assert!(result.new_addresses().next().is_none());

    let mut outputs = result.output_accounts();
    let mut inputs = result.input_accounts();

    // The result should contain combined data in ascending order (iters 1-10)
    for i in 1..=10 {
        assert_eq!(outputs.next().unwrap().lamports(), i as u64);
        assert_eq!(inputs.next().unwrap().lamports(), i as u64);
    }

    // Clear the CPI context account
    let cpi_context_cleared =
        deserialize_cpi_context_account_cleared(&cpi_context_account).unwrap();

    // Verify that the vectors are empty (their len should be 0)
    assert_eq!(cpi_context_cleared.new_addresses.len(), 0);
    assert_eq!(cpi_context_cleared.readonly_addresses.len(), 0);
    assert_eq!(cpi_context_cleared.readonly_accounts.len(), 0);
    assert_eq!(cpi_context_cleared.in_accounts.len(), 0);
    assert_eq!(cpi_context_cleared.out_accounts.len(), 0);
    assert_eq!(cpi_context_cleared.output_data.len(), 0);
    assert_eq!(cpi_context_cleared.output_data_len(), 0);
    // Fee payer should be reset to default
    assert_eq!(cpi_context_cleared.fee_payer.to_bytes(), Pubkey::default());

    // Assert raw bytes are zeroed (except discriminator, associated_merkle_tree, and vector capacities)
    assert_cpi_context_cleared_bytes(&cpi_context_account, merkle_tree_pubkey);

    let cpi_context = deserialize_cpi_context_account(&cpi_context_account).unwrap();

    assert_eq!(
        cpi_context.associated_merkle_tree.to_bytes(),
        merkle_tree_pubkey
    );
    assert_eq!(cpi_context.fee_payer.to_bytes(), Pubkey::default());
    assert!(cpi_context.is_empty());
}

/// Error: Invalid discriminator during deserialization
#[test]
fn test_deserialize_invalid_discriminator() {
    let account_info = get_account_info(
        solana_pubkey::Pubkey::new_unique().to_bytes(),
        ID,
        false,
        true,
        false,
        vec![0u8; 20000],
    );
    // Set invalid discriminator
    account_info.try_borrow_mut_data().unwrap()[0..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

    let result = deserialize_cpi_context_account(&account_info);
    assert_eq!(
        result.unwrap_err(),
        SystemProgramError::InvalidCpiContextDiscriminator.into()
    );
}

/// Error: Invalid owner for CPI context account
#[test]
fn test_deserialize_invalid_owner() {
    let wrong_owner = solana_pubkey::Pubkey::new_unique().to_bytes();
    let account_info = get_account_info(
        solana_pubkey::Pubkey::new_unique().to_bytes(),
        wrong_owner,
        false,
        true,
        false,
        vec![0u8; 20000],
    );

    let result = deserialize_cpi_context_account(&account_info);
    assert_eq!(
        result.unwrap_err(),
        SystemProgramError::InvalidCpiContextOwner.into()
    );
}

/// Error: RE_INIT with wrong discriminator
#[test]
fn test_cpi_context_reinit_wrong_discriminator() {
    let account_info = get_account_info(
        solana_pubkey::Pubkey::new_unique().to_bytes(),
        ID,
        false,
        true,
        false,
        vec![0u8; 20000],
    );
    // Set wrong discriminator (not V1)
    account_info.try_borrow_mut_data().unwrap()[0..8]
        .copy_from_slice(&[99, 98, 97, 96, 95, 94, 93, 92]);

    let params = CpiContextAccountInitParams::new(solana_pubkey::Pubkey::new_unique().to_bytes());
    let result = cpi_context_account_new::<true>(&account_info, params);
    assert_eq!(
        result.unwrap_err(),
        SystemProgramError::InvalidCpiContextDiscriminator.into()
    );
}

/// Error: Non-RE_INIT with non-zero discriminator
#[test]
fn test_cpi_context_init_nonzero_discriminator() {
    let account_info = get_account_info(
        solana_pubkey::Pubkey::new_unique().to_bytes(),
        ID,
        false,
        true,
        false,
        vec![0u8; 20000],
    );
    // Set non-zero discriminator
    account_info.try_borrow_mut_data().unwrap()[0..8].copy_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]);

    let params = CpiContextAccountInitParams::new(solana_pubkey::Pubkey::new_unique().to_bytes());
    let result = cpi_context_account_new::<false>(&account_info, params);
    assert_eq!(
        result.unwrap_err(),
        SystemProgramError::InvalidCpiContextDiscriminator.into()
    );
}

/// Randomized test for ZCpiContextAccount2 zero copy implementation
/// Tests:
/// 0. Create account bytes and init the account data (once)
/// Then 1000 iterations of:
/// 1. Create randomized input data
/// 2. Store the randomized data
/// 3. Deserialize again and assert the data
/// 4. Zero out
/// 5. Assert zeroed out
/// 6. Loop back to 1
#[test]
fn test_cpi_context_zero_copy_randomized() {
    // Use seeded RNG for reproducible tests
    let mut rng = StdRng::seed_from_u64(42);

    // 0. Create account bytes and init the account data (ONCE)
    let account_size = 20000; // Same size as used in other tests
    let account_data = vec![0u8; account_size];

    // Create random associated merkle tree
    let mut merkle_tree_bytes = [0u8; 32];
    rng.fill(&mut merkle_tree_bytes);
    let associated_merkle_tree = Pubkey::from(merkle_tree_bytes);

    // Fixed capacity values for the entire test
    let new_addresses_len = rng.gen_range(5..20);
    let readonly_addresses_len = rng.gen_range(5..20);
    let readonly_accounts_len = rng.gen_range(5..20);
    let in_accounts_len = rng.gen_range(10..30);
    let out_accounts_len = rng.gen_range(10..40);

    let params = CpiContextAccountInitParams {
        associated_merkle_tree,
        associated_queue: Pubkey::default(),
        new_addresses_len,
        readonly_addresses_len,
        readonly_accounts_len,
        in_accounts_len,
        out_accounts_len,
    };

    // Create mock account info
    let owner = ID;
    let mut key_bytes = [0u8; 32];
    rng.fill(&mut key_bytes);
    let account_info = get_account_info(
        Pubkey::from(key_bytes),
        owner,
        false,
        true,
        false,
        account_data,
    );

    // Initialize the account ONCE
    let _initial_context =
        cpi_context_account_new::<false>(&account_info, params).expect("Failed to init account");

    // Now loop 1000 times reusing the same account
    for iteration in 0..1000 {
        // Get the current context (after clearing it will be empty, ready for new data)
        let mut cpi_context = deserialize_cpi_context_account(&account_info)
            .unwrap_or_else(|_| panic!("Failed to deserialize at iteration {}", iteration));

        // 1. Create randomized input data
        let num_new_addresses = rng.gen_range(0..=new_addresses_len.min(5));
        let num_readonly_addresses = rng.gen_range(0..=readonly_addresses_len.min(5));
        let num_readonly_accounts = rng.gen_range(0..=readonly_accounts_len.min(5));
        let num_in_accounts = rng.gen_range(0..=in_accounts_len.min(10));
        let num_out_accounts = rng.gen_range(0..=out_accounts_len.min(10));

        // Generate random data for each type
        let mut expected_new_addresses = Vec::new();
        for _ in 0..num_new_addresses {
            let mut owner = [0u8; 32];
            let mut seed = [0u8; 32];
            rng.fill(&mut owner);
            rng.fill(&mut seed);

            expected_new_addresses.push(CpiContextNewAddressParamsAssignedPacked {
                owner,
                seed,
                address_queue_account_index: rng.gen(),
                address_merkle_tree_account_index: rng.gen(),
                address_merkle_tree_root_index: U16::new(rng.gen()),
                assigned_to_account: rng.gen_range(0..=1),
                assigned_account_index: rng.gen(),
            });
        }

        let mut expected_readonly_addresses = Vec::new();
        for _ in 0..num_readonly_addresses {
            let mut address = [0u8; 32];
            rng.fill(&mut address);
            expected_readonly_addresses.push(ZPackedReadOnlyAddress {
                address,
                address_merkle_tree_account_index: rng.gen(),
                address_merkle_tree_root_index: U16::new(rng.gen()),
            });
        }

        let mut expected_readonly_accounts = Vec::new();
        for _ in 0..num_readonly_accounts {
            let mut account_hash = [0u8; 32];
            rng.fill(&mut account_hash);

            expected_readonly_accounts.push(ZPackedReadOnlyCompressedAccount {
                account_hash,
                merkle_context: ZPackedMerkleContext {
                    merkle_tree_pubkey_index: rng.gen(),
                    queue_pubkey_index: rng.gen(),
                    leaf_index: U32::new(rng.gen()),
                    prove_by_index: rng.gen_range(0..=1),
                },
                root_index: U16::new(rng.gen()),
            });
        }

        let mut expected_in_accounts = Vec::new();
        for _ in 0..num_in_accounts {
            let mut owner = [0u8; 32];
            let mut discriminator = [0u8; 8];
            let mut data_hash = [0u8; 32];
            let mut address = [0u8; 32];
            rng.fill(&mut owner);
            rng.fill(&mut discriminator);
            rng.fill(&mut data_hash);
            rng.fill(&mut address);

            expected_in_accounts.push(CpiContextInAccount {
                owner: owner.into(),
                discriminator,
                data_hash,
                has_data: rng.gen(),
                merkle_context: ZPackedMerkleContext {
                    merkle_tree_pubkey_index: rng.gen(),
                    queue_pubkey_index: rng.gen(),
                    leaf_index: U32::new(rng.gen()),
                    prove_by_index: rng.gen_range(0..=1),
                },
                root_index: U16::new(rng.gen()),
                lamports: U64::new(rng.gen()),
                with_address: rng.gen_range(0..=1),
                address: if rng.gen_bool(0.5) { address } else { [0; 32] },
            });
        }

        let mut expected_out_accounts = Vec::new();
        for _ in 0..num_out_accounts {
            let mut owner = [0u8; 32];
            let mut discriminator = [0u8; 8];
            let mut data_hash = [0u8; 32];
            let mut address = [0u8; 32];
            rng.fill(&mut owner);
            rng.fill(&mut discriminator);
            rng.fill(&mut data_hash);
            rng.fill(&mut address);

            expected_out_accounts.push(CpiContextOutAccount {
                owner: owner.into(),
                discriminator,
                data_hash,
                has_data: rng.gen(),
                output_merkle_tree_index: rng.gen(),
                lamports: U64::new(rng.gen()),
                with_address: rng.gen_range(0..=1),
                address: if rng.gen_bool(0.5) { address } else { [0; 32] },
            });
        }

        // 2. Store the randomized data
        for item in &expected_new_addresses {
            cpi_context.new_addresses.push(*item).unwrap_or_else(|_| {
                panic!("Failed to push new address at iteration {}", iteration)
            });
        }

        for item in &expected_readonly_addresses {
            cpi_context
                .readonly_addresses
                .push(*item)
                .unwrap_or_else(|_| {
                    panic!("Failed to push readonly address at iteration {}", iteration)
                });
        }

        for item in &expected_readonly_accounts {
            cpi_context
                .readonly_accounts
                .push(*item)
                .unwrap_or_else(|_| {
                    panic!("Failed to push readonly account at iteration {}", iteration)
                });
        }

        for item in &expected_in_accounts {
            cpi_context
                .in_accounts
                .push(*item)
                .unwrap_or_else(|_| panic!("Failed to push in account at iteration {}", iteration));
        }

        for item in expected_out_accounts.iter() {
            cpi_context.out_accounts.push(*item).unwrap_or_else(|_| {
                panic!("Failed to push out account at iteration {}", iteration)
            });
        }

        // Note: We can't directly test output data storage as it requires WrappedInstructionData
        // which is part of the store_data API. The output data handling is tested indirectly
        // through the store_data tests in process_cpi_context.rs

        // 3. Deserialize again and assert the data
        let deserialized = deserialize_cpi_context_account(&account_info)
            .unwrap_or_else(|_| panic!("Failed to deserialize at iteration {}", iteration));

        // Assert all vectors have correct data
        assert_eq!(
            deserialized.new_addresses.len(),
            expected_new_addresses.len(),
            "new_addresses length mismatch at iteration {}",
            iteration
        );
        for (i, expected) in expected_new_addresses.iter().enumerate() {
            assert_eq!(
                *deserialized.new_addresses.get(i).unwrap(),
                *expected,
                "new_addresses[{}] mismatch at iteration {}",
                i,
                iteration
            );
        }

        assert_eq!(
            deserialized.readonly_addresses.len(),
            expected_readonly_addresses.len(),
            "readonly_addresses length mismatch at iteration {}",
            iteration
        );
        for (i, expected) in expected_readonly_addresses.iter().enumerate() {
            assert_eq!(
                *deserialized.readonly_addresses.get(i).unwrap(),
                *expected,
                "readonly_addresses[{}] mismatch at iteration {}",
                i,
                iteration
            );
        }

        assert_eq!(
            deserialized.readonly_accounts.len(),
            expected_readonly_accounts.len(),
            "readonly_accounts length mismatch at iteration {}",
            iteration
        );
        for (i, expected) in expected_readonly_accounts.iter().enumerate() {
            assert_eq!(
                *deserialized.readonly_accounts.get(i).unwrap(),
                *expected,
                "readonly_accounts[{}] mismatch at iteration {}",
                i,
                iteration
            );
        }

        assert_eq!(
            deserialized.in_accounts.len(),
            expected_in_accounts.len(),
            "in_accounts length mismatch at iteration {}",
            iteration
        );
        for (i, expected) in expected_in_accounts.iter().enumerate() {
            assert_eq!(
                *deserialized.in_accounts.get(i).unwrap(),
                *expected,
                "in_accounts[{}] mismatch at iteration {}",
                i,
                iteration
            );
        }

        assert_eq!(
            deserialized.out_accounts.len(),
            expected_out_accounts.len(),
            "out_accounts length mismatch at iteration {}",
            iteration
        );
        for (i, expected) in expected_out_accounts.iter().enumerate() {
            assert_eq!(
                *deserialized.out_accounts.get(i).unwrap(),
                *expected,
                "out_accounts[{}] mismatch at iteration {}",
                i,
                iteration
            );
        }

        // Output data is empty since we didn't use store_data API
        assert_eq!(
            deserialized.output_data_len(),
            0,
            "output_data_len should be 0 at iteration {}",
            iteration
        );
        assert_eq!(
            deserialized.output_data.len(),
            0,
            "output_data should be empty at iteration {}",
            iteration
        );

        // 4. Zero out (clear the account)
        let cleared = deserialize_cpi_context_account_cleared(&account_info)
            .unwrap_or_else(|_| panic!("Failed to deserialize cleared at iteration {}", iteration));

        // 5. Assert zeroed out
        assert_eq!(
            cleared.new_addresses.len(),
            0,
            "new_addresses not cleared at iteration {}",
            iteration
        );
        assert_eq!(
            cleared.readonly_addresses.len(),
            0,
            "readonly_addresses not cleared at iteration {}",
            iteration
        );
        assert_eq!(
            cleared.readonly_accounts.len(),
            0,
            "readonly_accounts not cleared at iteration {}",
            iteration
        );
        assert_eq!(
            cleared.in_accounts.len(),
            0,
            "in_accounts not cleared at iteration {}",
            iteration
        );
        assert_eq!(
            cleared.out_accounts.len(),
            0,
            "out_accounts not cleared at iteration {}",
            iteration
        );
        assert_eq!(
            cleared.output_data_len(),
            0,
            "output_data_len not cleared at iteration {}",
            iteration
        );
        assert_eq!(
            cleared.output_data.len(),
            0,
            "output_data not cleared at iteration {}",
            iteration
        );

        // Verify bytes are actually zeroed using the same approach as in process_cpi_context tests
        assert_cpi_context_cleared_bytes(&account_info, associated_merkle_tree);
    }

    println!("Successfully completed 1000 iterations of randomized zero-copy testing");
}
