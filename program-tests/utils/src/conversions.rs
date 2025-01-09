use light_compressed_token::{
    token_data::AccountState as ProgramAccountState, TokenData as ProgramTokenData,
};
use light_sdk::{self as sdk, proof::CompressedProof};
use light_system_program::{
    invoke::{
        processor::CompressedProof as ProgramCompressedProof,
        OutputCompressedAccountWithPackedContext as ProgramOutputCompressedAccountWithPackedContext,
    },
    sdk::{
        compressed_account::{
            CompressedAccount as ProgramCompressedAccount,
            CompressedAccountData as ProgramCompressedAccountData,
            CompressedAccountWithMerkleContext as ProgramCompressedAccountWithMerkleContext,
            MerkleContext as ProgramMerkleContext, QueueIndex as ProgramQueueIndex,
        },
        event::{
            MerkleTreeSequenceNumber as ProgramMerkleTreeSequenceNumber,
            PublicTransactionEvent as ProgramPublicTransactionEvent,
        },
    },
};

pub fn sdk_to_program_queue_index(
    sdk_queue_index: sdk::merkle_context::QueueIndex,
) -> ProgramQueueIndex {
    ProgramQueueIndex {
        queue_id: sdk_queue_index.queue_id,
        index: sdk_queue_index.index,
    }
}

pub fn program_to_sdk_queue_index(
    program_queue_index: ProgramQueueIndex,
) -> sdk::merkle_context::QueueIndex {
    sdk::merkle_context::QueueIndex {
        queue_id: program_queue_index.queue_id,
        index: program_queue_index.index,
    }
}

pub fn sdk_to_program_merkle_context(
    sdk_merkle_context: sdk::merkle_context::MerkleContext,
) -> ProgramMerkleContext {
    ProgramMerkleContext {
        merkle_tree_pubkey: sdk_merkle_context.merkle_tree_pubkey,
        nullifier_queue_pubkey: sdk_merkle_context.nullifier_queue_pubkey,
        leaf_index: sdk_merkle_context.leaf_index,
        queue_index: sdk_merkle_context
            .queue_index
            .map(sdk_to_program_queue_index),
    }
}

pub fn program_to_sdk_merkle_context(
    program_merkle_context: ProgramMerkleContext,
) -> sdk::merkle_context::MerkleContext {
    sdk::merkle_context::MerkleContext {
        merkle_tree_pubkey: program_merkle_context.merkle_tree_pubkey,
        nullifier_queue_pubkey: program_merkle_context.nullifier_queue_pubkey,
        leaf_index: program_merkle_context.leaf_index,
        queue_index: program_merkle_context
            .queue_index
            .map(program_to_sdk_queue_index),
    }
}
pub fn sdk_to_program_compressed_account_data(
    sdk_data: sdk::compressed_account::CompressedAccountData,
) -> ProgramCompressedAccountData {
    ProgramCompressedAccountData {
        discriminator: sdk_data.discriminator,
        data: sdk_data.data,
        data_hash: sdk_data.data_hash,
    }
}

pub fn program_to_sdk_compressed_account_data(
    program_data: ProgramCompressedAccountData,
) -> sdk::compressed_account::CompressedAccountData {
    sdk::compressed_account::CompressedAccountData {
        discriminator: program_data.discriminator,
        data: program_data.data,
        data_hash: program_data.data_hash,
    }
}

pub fn sdk_to_program_compressed_account(
    sdk_account: sdk::compressed_account::CompressedAccount,
) -> ProgramCompressedAccount {
    ProgramCompressedAccount {
        owner: sdk_account.owner,
        lamports: sdk_account.lamports,
        address: sdk_account.address,
        data: sdk_account.data.map(sdk_to_program_compressed_account_data),
    }
}

pub fn program_to_sdk_compressed_account(
    program_account: ProgramCompressedAccount,
) -> sdk::compressed_account::CompressedAccount {
    sdk::compressed_account::CompressedAccount {
        owner: program_account.owner,
        lamports: program_account.lamports,
        address: program_account.address,
        data: program_account
            .data
            .map(program_to_sdk_compressed_account_data),
    }
}

pub fn sdk_to_program_compressed_account_with_merkle_context(
    sdk_account: sdk::compressed_account::CompressedAccountWithMerkleContext,
) -> ProgramCompressedAccountWithMerkleContext {
    ProgramCompressedAccountWithMerkleContext {
        compressed_account: sdk_to_program_compressed_account(sdk_account.compressed_account),
        merkle_context: sdk_to_program_merkle_context(sdk_account.merkle_context),
    }
}

pub fn program_to_sdk_compressed_account_with_merkle_context(
    program_account: ProgramCompressedAccountWithMerkleContext,
) -> sdk::compressed_account::CompressedAccountWithMerkleContext {
    sdk::compressed_account::CompressedAccountWithMerkleContext {
        compressed_account: program_to_sdk_compressed_account(program_account.compressed_account),
        merkle_context: program_to_sdk_merkle_context(program_account.merkle_context),
    }
}

pub fn sdk_to_program_account_state(sdk_state: sdk::token::AccountState) -> ProgramAccountState {
    match sdk_state {
        sdk::token::AccountState::Initialized => ProgramAccountState::Initialized,
        sdk::token::AccountState::Frozen => ProgramAccountState::Frozen,
    }
}

pub fn program_to_sdk_account_state(
    program_state: ProgramAccountState,
) -> sdk::token::AccountState {
    match program_state {
        ProgramAccountState::Initialized => sdk::token::AccountState::Initialized,
        ProgramAccountState::Frozen => sdk::token::AccountState::Frozen,
    }
}

pub fn sdk_to_program_token_data(sdk_token: sdk::token::TokenData) -> ProgramTokenData {
    ProgramTokenData {
        mint: sdk_token.mint,
        owner: sdk_token.owner,
        amount: sdk_token.amount,
        delegate: sdk_token.delegate,
        state: sdk_to_program_account_state(sdk_token.state),
        tlv: sdk_token.tlv,
    }
}

pub fn program_to_sdk_token_data(program_token: ProgramTokenData) -> sdk::token::TokenData {
    sdk::token::TokenData {
        mint: program_token.mint,
        owner: program_token.owner,
        amount: program_token.amount,
        delegate: program_token.delegate,
        state: program_to_sdk_account_state(program_token.state),
        tlv: program_token.tlv,
    }
}

pub fn program_to_sdk_compressed_proof(program_proof: ProgramCompressedProof) -> CompressedProof {
    CompressedProof {
        a: program_proof.a,
        b: program_proof.b,
        c: program_proof.c,
    }
}

pub fn sdk_to_program_compressed_proof(sdk_proof: CompressedProof) -> ProgramCompressedProof {
    ProgramCompressedProof {
        a: sdk_proof.a,
        b: sdk_proof.b,
        c: sdk_proof.c,
    }
}

pub fn sdk_to_program_public_transaction_event(
    event: sdk::event::PublicTransactionEvent,
) -> ProgramPublicTransactionEvent {
    ProgramPublicTransactionEvent {
        input_compressed_account_hashes: event.input_compressed_account_hashes,
        output_compressed_account_hashes: event.output_compressed_account_hashes,
        output_compressed_accounts: event
            .output_compressed_accounts
            .into_iter()
            .map(|account| ProgramOutputCompressedAccountWithPackedContext {
                compressed_account: sdk_to_program_compressed_account(account.compressed_account),
                merkle_tree_index: account.merkle_tree_index,
            })
            .collect(),
        output_leaf_indices: event.output_leaf_indices,
        sequence_numbers: event
            .sequence_numbers
            .into_iter()
            .map(|sequence_number| ProgramMerkleTreeSequenceNumber {
                pubkey: sequence_number.pubkey,
                seq: sequence_number.seq,
            })
            .collect(),
        relay_fee: event.relay_fee,
        is_compress: event.is_compress,
        compress_or_decompress_lamports: event.compress_or_decompress_lamports,
        pubkey_array: event.pubkey_array,
        message: event.message,
    }
}

pub fn program_to_sdk_public_transaction_event(
    event: ProgramPublicTransactionEvent,
) -> sdk::event::PublicTransactionEvent {
    sdk::event::PublicTransactionEvent {
        input_compressed_account_hashes: event.input_compressed_account_hashes,
        output_compressed_account_hashes: event.output_compressed_account_hashes,
        output_compressed_accounts: event
            .output_compressed_accounts
            .into_iter()
            .map(
                |account| sdk::compressed_account::OutputCompressedAccountWithPackedContext {
                    compressed_account: program_to_sdk_compressed_account(
                        account.compressed_account,
                    ),
                    merkle_tree_index: account.merkle_tree_index,
                },
            )
            .collect(),
        output_leaf_indices: event.output_leaf_indices,
        sequence_numbers: event
            .sequence_numbers
            .into_iter()
            .map(|sequence_number| sdk::event::MerkleTreeSequenceNumber {
                pubkey: sequence_number.pubkey,
                seq: sequence_number.seq,
            })
            .collect(),
        relay_fee: event.relay_fee,
        is_compress: event.is_compress,
        compress_or_decompress_lamports: event.compress_or_decompress_lamports,
        pubkey_array: event.pubkey_array,
        message: event.message,
    }
}
