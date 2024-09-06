#![cfg(not(target_os = "solana"))]

use crate::escrow_with_compressed_pda::escrow::PackedInputCompressedPda;
use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressed_token::process_transfer::{
    get_cpi_authority_pda,
    transfer_sdk::{create_inputs_and_remaining_accounts_checked, to_account_metas},
    TokenTransferOutputData,
};
use light_sdk::{
    address::{NewAddressParams, NewAddressParamsPacked},
    merkle_context::{MerkleContext, PackedMerkleContext, QueueIndex},
    proof::CompressedProof,
    verify::CompressedCpiContext,
};
use light_system_program::sdk::{
    address::{add_and_get_remaining_account_indices, pack_new_address_params},
    compressed_account::{pack_merkle_context, CompressedAccount},
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

#[derive(Debug, Clone)]
pub struct CreateCompressedPdaEscrowInstructionInputs<'a> {
    pub lock_up_time: u64,
    pub signer: &'a Pubkey,
    pub input_merkle_context: &'a [MerkleContext],
    pub output_compressed_account_merkle_tree_pubkeys: &'a [Pubkey],
    pub output_compressed_accounts: &'a [TokenTransferOutputData],
    pub root_indices: &'a [u16],
    pub proof: &'a Option<CompressedProof>,
    pub input_token_data: &'a [light_compressed_token::token_data::TokenData],
    pub input_compressed_accounts: &'a [CompressedAccount],
    pub mint: &'a Pubkey,
    pub new_address_params: NewAddressParams,
    pub cpi_context_account: &'a Pubkey,
}

pub fn create_escrow_instruction(
    input_params: CreateCompressedPdaEscrowInstructionInputs,
    escrow_amount: u64,
) -> Instruction {
    // TODO(vadorovsky): Instead of doing this conversion, move all necessary
    // types from light-compressed-token into a separate crate.
    let input_merkle_context = input_params
        .input_merkle_context
        .iter()
        .map(
            |context| light_system_program::sdk::compressed_account::MerkleContext {
                merkle_tree_pubkey: context.merkle_tree_pubkey,
                nullifier_queue_pubkey: context.nullifier_queue_pubkey,
                leaf_index: context.leaf_index,
                queue_index: context.queue_index.map(|queue_index| {
                    light_system_program::sdk::compressed_account::QueueIndex {
                        queue_id: queue_index.queue_id,
                        index: queue_index.index,
                    }
                }),
            },
        )
        .collect::<Vec<_>>();

    let token_owner_pda = get_token_owner_pda(input_params.signer);

    // TODO(vadorovsky): Instead of doing this conversion, move all necessary
    // types from light-compressed-token into a separate crate.
    let proof = input_params.proof.as_ref().map(|proof| {
        light_system_program::invoke::processor::CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        }
    });

    let (mut remaining_accounts, inputs) = create_inputs_and_remaining_accounts_checked(
        input_params.input_token_data,
        input_params.input_compressed_accounts,
        input_merkle_context.as_slice(),
        None,
        input_params.output_compressed_accounts,
        input_params.root_indices,
        &proof,
        *input_params.mint,
        input_params.signer,
        false,
        None,
        None,
        None,
    )
    .unwrap();

    let merkle_tree_indices = add_and_get_remaining_account_indices(
        input_params.output_compressed_account_merkle_tree_pubkeys,
        &mut remaining_accounts,
    );

    // TODO(vadorovsky): Instead of doing this conversion, move all necessary
    // types from light-compressed-token into a separate crate.
    let new_address_params = light_system_program::invoke::instruction::NewAddressParams {
        seed: input_params.new_address_params.seed,
        address_queue_pubkey: input_params.new_address_params.address_queue_pubkey,
        address_merkle_tree_pubkey: input_params.new_address_params.address_merkle_tree_pubkey,
        address_merkle_tree_root_index: input_params
            .new_address_params
            .address_merkle_tree_root_index,
    };
    let new_address_params =
        pack_new_address_params(&[new_address_params], &mut remaining_accounts)[0];
    let new_address_params = NewAddressParamsPacked {
        seed: new_address_params.seed,
        address_queue_account_index: new_address_params.address_queue_account_index,
        address_merkle_tree_account_index: new_address_params.address_merkle_tree_account_index,
        address_merkle_tree_root_index: new_address_params.address_merkle_tree_root_index,
    };

    let cpi_context_account_index: u8 = match remaining_accounts
        .get(input_params.cpi_context_account)
    {
        Some(entry) => (*entry).try_into().unwrap(),
        None => {
            remaining_accounts.insert(*input_params.cpi_context_account, remaining_accounts.len());
            (remaining_accounts.len() - 1) as u8
        }
    };
    let instruction_data = crate::instruction::EscrowCompressedTokensWithCompressedPda {
        lock_up_time: input_params.lock_up_time,
        escrow_amount,
        proof: input_params.proof.clone().unwrap(),
        mint: *input_params.mint,
        signer_is_delegate: false,
        input_token_data_with_context: inputs.input_token_data_with_context,
        output_state_merkle_tree_account_indices: merkle_tree_indices,
        new_address_params,
        cpi_context: CompressedCpiContext {
            set_context: false,
            first_set_context: true,
            cpi_context_account_index,
        },
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[light_system_program::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = get_cpi_authority_pda().0;
    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);
    let cpi_authority_pda = light_sdk::utils::get_cpi_authority_pda(&crate::ID);

    let accounts = crate::accounts::EscrowCompressedTokensWithCompressedPda {
        signer: *input_params.signer,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        compressed_token_program: light_compressed_token::ID,
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        self_program: crate::ID,
        token_owner_pda: token_owner_pda.0,
        system_program: solana_sdk::system_program::id(),
        cpi_context_account: *input_params.cpi_context_account,
        cpi_authority_pda,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}

pub fn get_token_owner_pda(signer: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"escrow".as_ref(), signer.to_bytes().as_ref()],
        &crate::id(),
    )
}

#[derive(Debug, Clone)]
pub struct CreateCompressedPdaWithdrawalInstructionInputs<'a> {
    pub signer: &'a Pubkey,
    pub input_token_escrow_merkle_context: MerkleContext,
    pub input_cpda_merkle_context: MerkleContext,
    pub output_compressed_account_merkle_tree_pubkeys: &'a [Pubkey],
    pub output_compressed_accounts: &'a [TokenTransferOutputData],
    pub root_indices: &'a [u16],
    pub proof: &'a Option<CompressedProof>,
    pub input_token_data: &'a [light_compressed_token::token_data::TokenData],
    pub input_compressed_accounts: &'a [CompressedAccount],
    pub mint: &'a Pubkey,
    pub old_lock_up_time: u64,
    pub new_lock_up_time: u64,
    pub address: [u8; 32],
    pub cpi_context_account: &'a Pubkey,
}

pub fn create_withdrawal_instruction(
    input_params: CreateCompressedPdaWithdrawalInstructionInputs,
    withdrawal_amount: u64,
) -> Instruction {
    let (token_owner_pda, bump) = get_token_owner_pda(input_params.signer);

    // TODO(vadorovsky): Instead of doing these conversions, move all necessary
    // types from light-compressed-token into a separate crate.
    let input_cpda_merkle_context = light_system_program::sdk::compressed_account::MerkleContext {
        merkle_tree_pubkey: input_params.input_cpda_merkle_context.merkle_tree_pubkey,
        nullifier_queue_pubkey: input_params
            .input_cpda_merkle_context
            .nullifier_queue_pubkey,
        leaf_index: input_params.input_cpda_merkle_context.leaf_index,
        queue_index: input_params
            .input_cpda_merkle_context
            .queue_index
            .map(
                |queue_index| light_system_program::sdk::compressed_account::QueueIndex {
                    queue_id: queue_index.queue_id,
                    index: queue_index.index,
                },
            ),
    };
    let input_token_escrow_merkle_context =
        light_system_program::sdk::compressed_account::MerkleContext {
            merkle_tree_pubkey: input_params
                .input_token_escrow_merkle_context
                .merkle_tree_pubkey,
            nullifier_queue_pubkey: input_params
                .input_token_escrow_merkle_context
                .nullifier_queue_pubkey,
            leaf_index: input_params.input_token_escrow_merkle_context.leaf_index,
            queue_index: input_params
                .input_token_escrow_merkle_context
                .queue_index
                .map(
                    |queue_index| light_system_program::sdk::compressed_account::QueueIndex {
                        queue_id: queue_index.queue_id,
                        index: queue_index.index,
                    },
                ),
        };
    let proof = input_params.proof.as_ref().map(|proof| {
        light_system_program::invoke::processor::CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        }
    });

    let (mut remaining_accounts, inputs) = create_inputs_and_remaining_accounts_checked(
        input_params.input_token_data,
        input_params.input_compressed_accounts,
        &[input_token_escrow_merkle_context],
        None,
        input_params.output_compressed_accounts,
        input_params.root_indices,
        &proof,
        *input_params.mint,
        &token_owner_pda,
        false,
        None,
        None,
        None,
    )
    .unwrap();

    let merkle_tree_indices = add_and_get_remaining_account_indices(
        input_params.output_compressed_account_merkle_tree_pubkeys,
        &mut remaining_accounts,
    );

    let merkle_context_packed = pack_merkle_context(
        &[input_cpda_merkle_context, input_token_escrow_merkle_context],
        &mut remaining_accounts,
    );
    let merkle_context_packed = PackedMerkleContext {
        merkle_tree_pubkey_index: merkle_context_packed[0].merkle_tree_pubkey_index,
        nullifier_queue_pubkey_index: merkle_context_packed[0].nullifier_queue_pubkey_index,
        leaf_index: merkle_context_packed[0].leaf_index,
        queue_index: merkle_context_packed[0]
            .queue_index
            .map(|queue_index| QueueIndex {
                queue_id: queue_index.queue_id,
                index: queue_index.index,
            }),
    };

    let cpi_context_account_index: u8 = match remaining_accounts
        .get(input_params.cpi_context_account)
    {
        Some(entry) => (*entry).try_into().unwrap(),
        None => {
            remaining_accounts.insert(*input_params.cpi_context_account, remaining_accounts.len());
            (remaining_accounts.len() - 1) as u8
        }
    };
    let cpi_context = CompressedCpiContext {
        set_context: false,
        first_set_context: true,
        cpi_context_account_index,
    };
    let input_compressed_pda = PackedInputCompressedPda {
        old_lock_up_time: input_params.old_lock_up_time,
        new_lock_up_time: input_params.new_lock_up_time,
        address: input_params.address,
        merkle_context: merkle_context_packed,
        root_index: input_params.root_indices[0],
    };
    let instruction_data = crate::instruction::WithdrawCompressedTokensWithCompressedPda {
        proof: input_params.proof.clone().unwrap(),
        mint: *input_params.mint,
        signer_is_delegate: false,
        input_token_data_with_context: inputs.input_token_data_with_context,
        output_state_merkle_tree_account_indices: merkle_tree_indices,
        cpi_context,
        input_compressed_pda,
        withdrawal_amount,
        bump,
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[light_system_program::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = get_cpi_authority_pda().0;
    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);
    let cpi_authority_pda = light_system_program::utils::get_cpi_authority_pda(&crate::ID);

    let accounts = crate::accounts::EscrowCompressedTokensWithCompressedPda {
        signer: *input_params.signer,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        compressed_token_program: light_compressed_token::ID,
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        self_program: crate::ID,
        token_owner_pda,
        system_program: solana_sdk::system_program::id(),
        cpi_context_account: *input_params.cpi_context_account,
        cpi_authority_pda,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}
