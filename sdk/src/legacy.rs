//! Legacy types re-imported from programs which should be removed as soon as
//! possible.

pub use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        compressed_account::{
            CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext,
            PackedCompressedAccountWithMerkleContext, PackedMerkleContext, QueueIndex,
        },
        CompressedCpiContext,
    },
    InstructionDataInvokeCpi, NewAddressParams, NewAddressParamsPacked,
    OutputCompressedAccountWithPackedContext,
};
