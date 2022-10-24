use anchor_lang::{
    prelude::*,
};
use merkle_tree_program::{
    initialize_new_merkle_tree_18::PreInsertedLeavesIndex,
    RegisteredVerifier,
    poseidon_merkle_tree::state::MerkleTree
};
use merkle_tree_program::program::MerkleTreeProgram;
use anchor_spl::token::Token;

pub struct Accounts<'info, 'a, 'c> {
    pub program_id:         &'a Pubkey,
    pub signing_address:    AccountInfo<'info>,
    pub system_program:     &'a Program<'info, System>,
    pub program_merkle_tree: &'a Program<'info, MerkleTreeProgram>,
    pub rent:               &'a Sysvar<'info, Rent>,
    pub merkle_tree:        &'a AccountLoader<'info, MerkleTree>,
    pub pre_inserted_leaves_index: &'a Account<'info, PreInsertedLeavesIndex>,
    pub authority:          AccountInfo<'info>,
    pub token_program:      Option<&'a Program<'info, Token>>,
    pub sender:             Option<AccountInfo<'info>>,
    pub recipient:          Option<AccountInfo<'info>>,
    pub sender_fee:         Option<AccountInfo<'info>>,
    pub recipient_fee:      Option<AccountInfo<'info>>,
    pub relayer_recipient:  Option<AccountInfo<'info>>,
    pub escrow:             Option<AccountInfo<'info>>,
    pub token_authority:    Option<AccountInfo<'info>>,
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
        token_program:      Option<&'a Program<'info, Token>>,
        sender:             Option<AccountInfo<'info>>,
        recipient:          Option<AccountInfo<'info>>,
        sender_fee:         Option<AccountInfo<'info>>,
        recipient_fee:      Option<AccountInfo<'info>>,
        relayer_recipient:  Option<AccountInfo<'info>>,
        escrow:             Option<AccountInfo<'info>>,
        token_authority:    Option<AccountInfo<'info>>,
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
