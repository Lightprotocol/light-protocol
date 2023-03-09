use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use merkle_tree_program::program::MerkleTreeProgram;
use merkle_tree_program::{
    transaction_merkle_tree::state::TransactionMerkleTree, RegisteredVerifier,
};

pub struct Accounts<'info, 'a, 'c> {
    pub program_id: &'a Pubkey,
    pub signing_address: AccountInfo<'info>,
    pub system_program: &'a Program<'info, System>,
    pub program_merkle_tree: &'a Program<'info, MerkleTreeProgram>,
    pub transaction_merkle_tree: &'a AccountLoader<'info, TransactionMerkleTree>,
    pub authority: AccountInfo<'info>,
    pub token_program: Option<&'a Program<'info, Token>>,
    pub sender: Option<AccountInfo<'info>>,
    pub recipient: Option<AccountInfo<'info>>,
    pub sender_fee: Option<AccountInfo<'info>>,
    pub recipient_fee: Option<AccountInfo<'info>>,
    pub relayer_recipient: Option<AccountInfo<'info>>,
    pub token_authority: Option<AccountInfo<'info>>,
    pub registered_verifier_pda: &'a Account<'info, RegisteredVerifier>,
    pub remaining_accounts: &'c [AccountInfo<'info>],
}

impl<'info, 'a, 'c> Accounts<'info, 'a, 'c> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        program_id: &'a Pubkey,
        signing_address: AccountInfo<'info>,
        system_program: &'a Program<'info, System>,
        program_merkle_tree: &'a Program<'info, MerkleTreeProgram>,
        transaction_merkle_tree: &'a AccountLoader<'info, TransactionMerkleTree>,
        authority: AccountInfo<'info>,
        token_program: Option<&'a Program<'info, Token>>,
        sender: Option<AccountInfo<'info>>,
        recipient: Option<AccountInfo<'info>>,
        sender_fee: Option<AccountInfo<'info>>,
        recipient_fee: Option<AccountInfo<'info>>,
        relayer_recipient: Option<AccountInfo<'info>>,
        token_authority: Option<AccountInfo<'info>>,
        registered_verifier_pda: &'a Account<'info, RegisteredVerifier>,
        remaining_accounts: &'c [AccountInfo<'info>],
    ) -> Result<Self> {
        Ok(Self {
            program_id,
            signing_address,
            system_program,
            program_merkle_tree,
            transaction_merkle_tree,
            authority,
            token_program,
            sender,
            recipient,
            sender_fee,
            recipient_fee,
            relayer_recipient,
            token_authority,
            registered_verifier_pda,
            remaining_accounts,
        })
    }
}
