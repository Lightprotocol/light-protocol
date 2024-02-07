use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use light_merkle_tree_program::{
    program::LightMerkleTreeProgram, state::MerkleTreeSet, RegisteredVerifier,
};

pub trait LightAccounts<'info> {
    fn get_signing_address(&self) -> &Signer<'info>;
    fn get_system_program(&self) -> &Program<'info, System>;
    fn get_program_merkle_tree(&self) -> &Program<'info, LightMerkleTreeProgram>;
    fn get_merkle_tree_set(&self) -> Option<&AccountLoader<'info, MerkleTreeSet>>;
    fn get_authority(&self) -> &UncheckedAccount<'info>;
    fn get_rpc_recipient_sol(&self) -> &UncheckedAccount<'info>;
    fn get_registered_verifier_pda(&self) -> &Account<'info, RegisteredVerifier>;
    fn get_sender_sol(&self) -> Option<&UncheckedAccount<'info>>;
    fn get_recipient_sol(&self) -> Option<&UncheckedAccount<'info>>;
    fn get_token_program(&self) -> Option<&Program<'info, Token>>;
    fn get_token_authority(&self) -> Option<&AccountInfo<'info>>;
    fn get_sender_spl(&self) -> Option<&UncheckedAccount<'info>>;
    fn get_recipient_spl(&self) -> Option<&UncheckedAccount<'info>>;
    fn get_log_wrapper(&self) -> &UncheckedAccount<'info>;
    fn get_account_compression_authority(&self) -> Option<&UncheckedAccount<'info>>;
    fn get_psp_account_compression_program(&self) -> Option<&Program<'info, AccountCompression>>;
}

/// merkle tree state accounts are in remaining accounts first in Merkle trees then out Merkle trees
pub trait LightPublicAccounts<'info> {
    fn get_signing_address(&self) -> &Signer<'info>;
    fn get_system_program(&self) -> &Program<'info, System>;
    fn get_program_merkle_tree(&self) -> &Program<'info, LightMerkleTreeProgram>;
    fn get_authority(&self) -> &UncheckedAccount<'info>;
    fn get_psp_account_compression_program(&self) -> &Program<'info, AccountCompression>;
    fn get_account_compression_authority(&self) -> &UncheckedAccount<'info>;
    fn get_rpc_recipient_sol(&self) -> &UncheckedAccount<'info>;
    fn get_registered_verifier_pda(&self) -> &Account<'info, RegisteredVerifier>;
    fn get_sender_sol(&self) -> Option<&UncheckedAccount<'info>>;
    fn get_recipient_sol(&self) -> Option<&UncheckedAccount<'info>>;
    fn get_token_program(&self) -> Option<&Program<'info, Token>>;
    fn get_token_authority(&self) -> Option<&AccountInfo<'info>>;
    fn get_sender_spl(&self) -> Option<&UncheckedAccount<'info>>;
    fn get_recipient_spl(&self) -> Option<&UncheckedAccount<'info>>;
    fn get_log_wrapper(&self) -> &UncheckedAccount<'info>;
}
