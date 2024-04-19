use crate::{
    append_state::insert_output_compressed_accounts_into_state_merkle_tree,
    compressed_account::{derive_address, CompressedAccount, CompressedAccountWithMerkleContext},
    compressed_cpi::{process_cpi_context, CompressedCpiContext, CpiSignatureAccount},
    compression_lamports,
    create_address::insert_addresses_into_address_merkle_tree_queue,
    event::{emit_state_transition_event, PublicTransactionEvent},
    nullify_state::insert_nullifiers,
    utils::CompressedProof,
    verify_state::{
        fetch_roots, fetch_roots_address_merkle_tree, hash_input_compressed_accounts, signer_check,
        sum_check, verify_state_proof, write_access_check,
    },
    CompressedSolPda, ErrorCode,
};
use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;

pub fn process_execute_compressed_transaction<'a, 'b, 'c: 'info, 'info>(
    inputs: &mut InstructionDataTransfer,
    mut ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    cpi_context: Option<CompressedCpiContext>,
) -> anchor_lang::Result<PublicTransactionEvent> {
    // signer check ---------------------------------------------------
    signer_check(&inputs, &ctx)?;
    write_access_check(&inputs, &ctx.accounts.invoking_program)?;
    if let Some(cpi_context) = cpi_context {
        let res = process_cpi_context(cpi_context, &mut ctx, inputs)?;
        match res {
            Some(event) => return event,
            None => {}
        }
    }
    // TODO: if execute and cpi_signature_account combine stored inputs with current inputs
    // sum check ---------------------------------------------------
    // the sum of in compressed accounts and compressed accounts must be equal minus the relay fee
    sum_check(
        &inputs.input_compressed_accounts_with_merkle_context,
        &inputs.output_compressed_accounts,
        &inputs.relay_fee,
        &inputs.compression_lamports,
        &inputs.is_compress,
    )?;
    msg!("sum check success");
    // compression_lamports ---------------------------------------------------
    compression_lamports(&inputs, &ctx)?;

    let mut roots = vec![[0u8; 32]; inputs.input_compressed_accounts_with_merkle_context.len()];
    fetch_roots(&inputs, &ctx, &mut roots)?;
    let mut address_roots = vec![[0u8; 32]; inputs.new_address_params.len()];
    // TODO: enable once address merkle tree init is debugged
    fetch_roots_address_merkle_tree(&inputs, &ctx, &mut address_roots)?;

    let mut input_compressed_account_hashes =
        vec![[0u8; 32]; inputs.input_compressed_accounts_with_merkle_context.len()];
    let mut input_compressed_account_addresses: Vec<Option<[u8; 32]>> =
        vec![None; inputs.input_compressed_accounts_with_merkle_context.len()];

    let mut output_leaf_indices = vec![0u32; inputs.output_compressed_accounts.len()];
    let mut output_compressed_account_hashes =
        vec![[0u8; 32]; inputs.output_compressed_accounts.len()];
    let mut new_addresses = vec![[0u8; 32]; inputs.new_address_params.len()];
    // insert addresses into address merkle tree queue ---------------------------------------------------
    if !new_addresses.is_empty() {
        derive_new_addresses(
            &inputs,
            &ctx,
            &mut input_compressed_account_addresses,
            &mut new_addresses,
        );
        insert_addresses_into_address_merkle_tree_queue(&inputs, &ctx, &new_addresses)?;
    }
    // TODO: add heap neutral
    hash_input_compressed_accounts(
        &ctx,
        &inputs,
        &mut input_compressed_account_hashes,
        &mut input_compressed_account_addresses,
    )?;

    // TODO: add heap neutral
    // verify inclusion proof ---------------------------------------------------
    if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
        || !new_addresses.is_empty()
    {
        verify_state_proof(
            &roots,
            &input_compressed_account_hashes,
            &address_roots,
            new_addresses.as_slice(),
            inputs.proof.as_ref().unwrap(),
        )?;
    }
    // insert nullifiers (input compressed account hashes)---------------------------------------------------
    if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
    {
        insert_nullifiers(&inputs, &ctx, &input_compressed_account_hashes)?;
    }

    // insert leaves (output compressed account hashes) ---------------------------------------------------
    if !inputs.output_compressed_accounts.is_empty() {
        insert_output_compressed_accounts_into_state_merkle_tree(
            &inputs,
            &ctx,
            &mut output_leaf_indices,
            &mut output_compressed_account_hashes,
            &mut input_compressed_account_addresses,
        )?;
    }

    // emit state transition event ---------------------------------------------------
    emit_state_transition_event(
        &inputs,
        &ctx,
        &input_compressed_account_hashes,
        &output_compressed_account_hashes,
        &output_leaf_indices,
    )
}

// DO NOT MAKE HEAP NEUTRAL: this function allocates new heap memory
pub fn derive_new_addresses(
    inputs: &InstructionDataTransfer,
    ctx: &Context<'_, '_, '_, '_, TransferInstruction<'_>>,
    input_compressed_account_addresses: &mut Vec<Option<[u8; 32]>>,
    new_addresses: &mut [[u8; 32]],
) {
    inputs
        .new_address_params
        .iter()
        .enumerate()
        .for_each(|(i, new_address_params)| {
            let address = derive_address(
                &ctx.remaining_accounts
                    [new_address_params.address_merkle_tree_account_index as usize]
                    .key(),
                &new_address_params.seed,
            )
            .unwrap();
            input_compressed_account_addresses.push(Some(address));
            new_addresses[i] = address;
        });
}

/// These are the base accounts additionally Merkle tree and queue accounts are required.
/// These additional accounts are passed as remaining accounts.
/// 1 Merkle tree for each input compressed account one queue and Merkle tree account each for each output compressed account.
#[derive(Accounts)]
pub struct TransferInstruction<'info> {
    pub signer: Signer<'info>,
    /// CHECK: this account
    #[account(
    seeds = [&crate::ID.to_bytes()], bump, seeds::program = &account_compression::ID,
    )]
    pub registered_program_pda:
        Account<'info, account_compression::instructions::register_program::RegisteredProgram>,
    /// CHECK: this account
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    #[account(seeds = [b"cpi_authority"], bump,)]
    pub psp_account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    pub account_compression_program: Program<'info, AccountCompression>,
    #[account(mut)]
    pub cpi_signature_account: Option<Account<'info, CpiSignatureAccount>>,
    pub invoking_program: Option<UncheckedAccount<'info>>,
    #[account(mut)]
    pub compressed_sol_pda: Option<Account<'info, CompressedSolPda>>,
    #[account(mut)]
    pub compression_recipient: Option<UncheckedAccount<'info>>,
    pub system_program: Option<Program<'info, System>>,
}

// TODO: add checks for lengths of vectors
#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InstructionDataTransfer {
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub input_root_indices: Vec<u16>,
    pub input_compressed_accounts_with_merkle_context: Vec<CompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<CompressedAccount>,
    /// The indices of the accounts in the output state merkle tree.
    pub output_state_merkle_tree_account_indices: Vec<u8>,
    pub relay_fee: Option<u64>,
    pub compression_lamports: Option<u64>,
    pub is_compress: bool,
    pub signer_seeds: Option<Vec<Vec<u8>>>,
}

impl InstructionDataTransfer {
    pub fn combine(&mut self, other: &[InstructionDataTransfer]) {
        for other in other {
            self.new_address_params
                .extend_from_slice(&other.new_address_params);
            self.input_root_indices
                .extend_from_slice(&other.input_root_indices);
            self.input_compressed_accounts_with_merkle_context
                .extend_from_slice(&other.input_compressed_accounts_with_merkle_context);
            self.output_compressed_accounts
                .extend_from_slice(&other.output_compressed_accounts);
            self.output_state_merkle_tree_account_indices
                .extend_from_slice(&other.output_state_merkle_tree_account_indices);
        }
    }
}

// test combine instruction data transfer
#[test]
fn test_combine_instruction_data_transfer() {
    let mut instruction_data_transfer = InstructionDataTransfer {
        proof: Some(CompressedProof {
            a: [0; 32],
            b: [0; 64],
            c: [0; 32],
        }),
        new_address_params: vec![NewAddressParamsPacked::default()],
        input_root_indices: vec![1],
        input_compressed_accounts_with_merkle_context: vec![
            CompressedAccountWithMerkleContext::default(),
        ],
        output_compressed_accounts: vec![CompressedAccount::default()],
        output_state_merkle_tree_account_indices: vec![1],
        relay_fee: Some(1),
        compression_lamports: Some(1),
        is_compress: true,
        signer_seeds: None,
    };
    let other = InstructionDataTransfer {
        proof: Some(CompressedProof {
            a: [0; 32],
            b: [0; 64],
            c: [0; 32],
        }),
        new_address_params: vec![NewAddressParamsPacked::default()],
        input_root_indices: vec![1],
        input_compressed_accounts_with_merkle_context: vec![
            CompressedAccountWithMerkleContext::default(),
        ],
        output_compressed_accounts: vec![CompressedAccount::default()],
        output_state_merkle_tree_account_indices: vec![1],
        relay_fee: Some(1),
        compression_lamports: Some(1),
        is_compress: true,
        signer_seeds: None,
    };
    instruction_data_transfer.combine(&[other]);
    assert_eq!(instruction_data_transfer.new_address_params.len(), 2);
    assert_eq!(instruction_data_transfer.input_root_indices.len(), 2);
    assert_eq!(
        instruction_data_transfer
            .input_compressed_accounts_with_merkle_context
            .len(),
        2
    );
    assert_eq!(
        instruction_data_transfer.output_compressed_accounts.len(),
        2
    );
    assert_eq!(
        instruction_data_transfer
            .output_state_merkle_tree_account_indices
            .len(),
        2
    );
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct NewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct NewAddressParams {
    pub seed: [u8; 32],
    pub address_queue_pubkey: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
}

impl InstructionDataTransfer {
    /// Checks that the lengths of the vectors are consistent with each other.
    /// Note that this function does not check the inputs themselves just plausible of the lengths.
    /// input roots must be the same length as input compressed accounts
    /// output compressed accounts must be the same length as output state merkle tree account indices
    pub fn check_input_lengths(&self) -> Result<()> {
        if self.input_root_indices.len() != self.input_compressed_accounts_with_merkle_context.len()
        {
            msg!("input_root_indices.len() {} != {} input_compressed_accounts_with_merkle_context.len()", 
                self.input_root_indices.len(), self.input_compressed_accounts_with_merkle_context.len()
            );
            msg!("self {:?}", self);
            return Err(ErrorCode::LengthMismatch.into());
        }

        if self.output_compressed_accounts.len()
            != self.output_state_merkle_tree_account_indices.len()
        {
            msg!("output_compressed_accounts.len() {} != {} output_state_merkle_tree_account_indices.len()", 
                self.output_compressed_accounts.len(), self.output_state_merkle_tree_account_indices.len()
            );
            msg!("self {:?}", self);
            return Err(ErrorCode::LengthMismatch.into());
        }

        Ok(())
    }
}

// TODO: refactor to compressed_account
// #[derive(Debug)]
// #[account]
// pub struct InstructionDataTransfer2 {
//     pub proof: Option<CompressedProof>,
//     pub low_element_indices: Vec<u16>,
//     pub root_indices: Vec<u16>,
//     pub relay_fee: Option<u64>,
//     pub utxos: SerializedUtxos,
// }

// pub fn into_inputs(
//     inputs: InstructionDataTransfer2,
//     accounts: &[Pubkey],
//     remaining_accounts: &[Pubkey],
// ) -> Result<InstructionDataTransfer> {
//     let input_compressed_accounts_with_merkle_context = inputs
//         .utxos
//         .input_compressed_accounts_from_serialized_utxos(accounts, remaining_accounts)
//         .unwrap();
//     let output_compressed_accounts = inputs
//         .utxos
//         .output_compressed_accounts_from_serialized_utxos(accounts)
//         .unwrap();
//     Ok(InstructionDataTransfer {
//         proof: inputs.proof,
//         low_element_indices: inputs.low_element_indices,
//         root_indices: inputs.root_indices,
//         relay_fee: inputs.relay_fee,
//         input_compressed_accounts_with_merkle_context,
//         output_compressed_accounts,
//     })
// }
