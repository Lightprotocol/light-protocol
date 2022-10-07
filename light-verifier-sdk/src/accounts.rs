use anchor_lang::{
    prelude::*,
    solana_program::{
        self
    }
};
use merkle_tree_program::{
    initialize_new_merkle_tree_18::PreInsertedLeavesIndex,
    RegisteredVerifier,
    poseidon_merkle_tree::state::MerkleTree
};
use merkle_tree_program::program::MerkleTreeProgram;
use anchor_spl::token::Token;
use crate::errors::VerifierSdkError;
use std::cell::Ref;

pub struct Accounts<'info, 'a, 'c> {
    pub program_id:         &'a Pubkey,
    pub signing_address:    AccountInfo<'info>,
    pub system_program:     &'a Program<'info, System>,
    pub program_merkle_tree: &'a Program<'info, MerkleTreeProgram>,
    pub rent:               &'a Sysvar<'info, Rent>,
    pub merkle_tree:        &'a AccountLoader<'info, MerkleTree>,
    pub pre_inserted_leaves_index: &'a Account<'info, PreInsertedLeavesIndex>,
    pub authority:          AccountInfo<'info>,
    pub token_program:      &'a Program<'info, Token>,
    pub sender:             AccountInfo<'info>,
    pub recipient:          AccountInfo<'info>,
    pub sender_fee:         AccountInfo<'info>,
    pub recipient_fee:      AccountInfo<'info>,
    pub relayer_recipient:  AccountInfo<'info>,
    pub escrow:             AccountInfo<'info>,
    pub token_authority:    AccountInfo<'info>,
    pub registered_verifier_pda: &'a Account<'info, RegisteredVerifier>,
    pub remaining_accounts: &'c [AccountInfo<'info>]
}

impl <'info, 'a, 'c>Accounts<'info, 'a, 'c> {

    pub fn new(
        program_id:         &'a Pubkey,
        signing_address:    AccountInfo<'info>,
        system_program:     &'a Program<'info, System>,
        program_merkle_tree: &'a Program<'info, MerkleTreeProgram>,
        rent:               &'a Sysvar<'info, Rent>,
        merkle_tree:        &'a AccountLoader<'info, MerkleTree>,
        pre_inserted_leaves_index: &'a Account<'info, PreInsertedLeavesIndex>,
        authority:          AccountInfo<'info>,
        token_program:      &'a Program<'info, Token>,
        sender:             AccountInfo<'info>,
        recipient:          AccountInfo<'info>,
        sender_fee:         AccountInfo<'info>,
        recipient_fee:      AccountInfo<'info>,
        relayer_recipient:  AccountInfo<'info>,
        escrow:             AccountInfo<'info>,
        token_authority:    AccountInfo<'info>,
        registered_verifier_pda: &'a Account<'info, RegisteredVerifier>,
        remaining_accounts: &'c [AccountInfo<'info>],
    ) -> Result<Self> {

    return Ok(Self {
        program_id: program_id,
        signing_address: signing_address,
        system_program: system_program,
        program_merkle_tree: program_merkle_tree,
        rent: rent,
        merkle_tree: merkle_tree,
        pre_inserted_leaves_index: pre_inserted_leaves_index,
        authority: authority,
        token_program: token_program,
        sender: sender,
        recipient: recipient,
        sender_fee: sender_fee,
        recipient_fee: recipient_fee,
        relayer_recipient: relayer_recipient,
        escrow: escrow,
        token_authority: token_authority,
        registered_verifier_pda: registered_verifier_pda,
        remaining_accounts: remaining_accounts
    });
    }
}
