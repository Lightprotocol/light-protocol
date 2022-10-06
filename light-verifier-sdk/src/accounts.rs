use anchor_lang::{
    prelude::*,
    solana_program::{
        self
    }
};
use errors::VerifierSdkError;
use light_transaction::TxConfig;

pub struct Accounts<'info, 'a, 'c, T: TxConfig> {
    pub program_id: &'a Pubkey,
    pub signing_address:  AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub program_merkle_tree: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
    pub merkle_tree: AccountInfo<'info>,
    pub pre_inserted_leaves_index: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub sender: AccountInfo<'info>,
    pub recipient: AccountInfo<'info>,
    pub sender_fee: AccountInfo<'info>,
    pub recipient_fee: AccountInfo<'info>,
    pub relayer_recipient: AccountInfo<'info>,
    pub escrow: AccountInfo<'info>,
    pub token_authority: AccountInfo<'info>,
    pub remaining_accounts: &'c [AccountInfo<'info>]
}

impl <T: TxConfig>Accounts<'_, '_, '_, T> {

    pub fn new<'info, 'a, 'c> (
        program_id: &'a Pubkey,
        signing_address:  AccountInfo<'info>,
        system_program: AccountInfo<'info>,
        program_merkle_tree: AccountInfo<'info>,
        rent: AccountInfo<'info>,
        merkle_tree: AccountInfo<'info>,
        pre_inserted_leaves_index: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        sender: AccountInfo<'info>,
        recipient: AccountInfo<'info>,
        sender_fee: AccountInfo<'info>,
        recipient_fee: AccountInfo<'info>,
        relayer_recipient: AccountInfo<'info>,
        escrow: AccountInfo<'info>,
        token_authority: AccountInfo<'info>,
        remaining_accounts: &'c [AccountInfo<'info>]
    ) -> Result<Self, VerifierSdkError> {
            
    return Self {
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
        remaining_accounts: remaining_accounts
    };
    }
}
