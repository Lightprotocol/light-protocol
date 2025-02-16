use light_compressed_token::{
    token_data::AccountState as ProgramAccountState, TokenData as ProgramTokenData,
};
use light_sdk::{self as sdk};

// pub fn sdk_to_program_merkle_context(
//     sdk_merkle_context: sdk::merkle_context::MerkleContext,
// ) -> ProgramMerkleContext {
//     ProgramMerkleContext {
//         merkle_tree_pubkey: sdk_merkle_context.merkle_tree_pubkey,
//         nullifier_queue_pubkey: sdk_merkle_context.nullifier_queue_pubkey,
//         leaf_index: sdk_merkle_context.leaf_index,
//         prove_by_index: sdk_merkle_context.prove_by_index,
//     }
// }

// pub fn program_to_sdk_merkle_context(
//     program_merkle_context: ProgramMerkleContext,
// ) -> sdk::merkle_context::MerkleContext {
//     sdk::merkle_context::MerkleContext {
//         merkle_tree_pubkey: program_merkle_context.merkle_tree_pubkey,
//         nullifier_queue_pubkey: program_merkle_context.nullifier_queue_pubkey,
//         leaf_index: program_merkle_context.leaf_index,
//         prove_by_index: program_merkle_context.prove_by_index,
//     }
// }
// pub fn sdk_to_program_compressed_account_data(
//     sdk_data: sdk::compressed_account::CompressedAccountData,
// ) -> ProgramCompressedAccountData {
//     ProgramCompressedAccountData {
//         discriminator: sdk_data.discriminator,
//         data: sdk_data.data,
//         data_hash: sdk_data.data_hash,
//     }
// }

// pub fn program_to_sdk_compressed_account_data(
//     program_data: ProgramCompressedAccountData,
// ) -> sdk::compressed_account::CompressedAccountData {
//     sdk::compressed_account::CompressedAccountData {
//         discriminator: program_data.discriminator,
//         data: program_data.data,
//         data_hash: program_data.data_hash,
//     }
// }

// pub fn sdk_to_program_compressed_account(
//     sdk_account: sdk::compressed_account::CompressedAccount,
// ) -> ProgramCompressedAccount {
//     ProgramCompressedAccount {
//         owner: sdk_account.owner,
//         lamports: sdk_account.lamports,
//         address: sdk_account.address,
//         data: sdk_account.data.map(sdk_to_program_compressed_account_data),
//     }
// }

// pub fn program_to_sdk_compressed_account(
//     program_account: ProgramCompressedAccount,
// ) -> sdk::compressed_account::CompressedAccount {
//     sdk::compressed_account::CompressedAccount {
//         owner: program_account.owner,
//         lamports: program_account.lamports,
//         address: program_account.address,
//         data: program_account
//             .data
//             .map(program_to_sdk_compressed_account_data),
//     }
// }

// pub fn sdk_to_program_compressed_account_with_merkle_context(
//     sdk_account: sdk::compressed_account::CompressedAccountWithMerkleContext,
// ) -> ProgramCompressedAccountWithMerkleContext {
//     ProgramCompressedAccountWithMerkleContext {
//         compressed_account: sdk_to_program_compressed_account(sdk_account.compressed_account),
//         merkle_context: sdk_to_program_merkle_context(sdk_account.merkle_context),
//     }
// }

// pub fn program_to_sdk_compressed_account_with_merkle_context(
//     program_account: ProgramCompressedAccountWithMerkleContext,
// ) -> sdk::compressed_account::CompressedAccountWithMerkleContext {
//     sdk::compressed_account::CompressedAccountWithMerkleContext {
//         compressed_account: program_to_sdk_compressed_account(program_account.compressed_account),
//         merkle_context: program_to_sdk_merkle_context(program_account.merkle_context),
//     }
// }

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
