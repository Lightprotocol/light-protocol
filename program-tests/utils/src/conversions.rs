use light_ctoken_types::state::{CompressedTokenAccountState, TokenData as ProgramTokenData};
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

pub fn sdk_to_program_account_state(
    sdk_state: sdk::token::AccountState,
) -> CompressedTokenAccountState {
    match sdk_state {
        sdk::token::AccountState::Initialized => CompressedTokenAccountState::Initialized,
        sdk::token::AccountState::Frozen => CompressedTokenAccountState::Frozen,
    }
}

pub fn program_to_sdk_account_state(program_state: u8) -> sdk::token::AccountState {
    match program_state {
        0 => sdk::token::AccountState::Initialized,
        1 => sdk::token::AccountState::Frozen,
        _ => panic!("program_to_sdk_account_state: invalid account state"),
    }
}

pub fn sdk_to_program_token_data(sdk_token: sdk::token::TokenData) -> ProgramTokenData {
    ProgramTokenData {
        mint: sdk_token.mint.into(),
        owner: sdk_token.owner.into(),
        amount: sdk_token.amount,
        delegate: sdk_token.delegate.map(|d| d.into()),
        state: sdk_to_program_account_state(sdk_token.state) as u8,
        tlv: sdk_token.tlv,
    }
}

pub fn program_to_sdk_token_data(program_token: ProgramTokenData) -> sdk::token::TokenData {
    sdk::token::TokenData {
        mint: program_token.mint.into(),
        owner: program_token.owner.into(),
        amount: program_token.amount,
        delegate: program_token.delegate.map(|d| d.into()),
        state: program_to_sdk_account_state(program_token.state),
        tlv: program_token.tlv,
    }
}
