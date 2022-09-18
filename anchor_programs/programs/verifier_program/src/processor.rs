use anchor_spl::token::Token;
use anchor_lang::prelude::*;
use merkle_tree_program::initialize_new_merkle_tree_spl::PreInsertedLeavesIndex;
use merkle_tree_program::program::MerkleTreeProgram;
use solana_program::log::sol_log_compute_units;
use crate::LightTransaction;

#[derive( Accounts)]
#[instruction(
    proof:              [u8;256],
    merkle_root:        [u8;32],
    amount:             [u8;32],
    tx_integrity_hash:  [u8;32]
)]
pub struct ShieldedTransfer2Inputs<'info> {
    // #[account(init_if_needed, seeds = [tx_integrity_hash.as_ref(), b"storage"], bump,  payer=signing_address, space= 5 * 1024)]
    // pub verifier_state: AccountLoader<'info, VerifierState>,
    /// First time therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: Is the same as in integrity hash.
    // #[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[usize::try_from(self.load()?.merkle_tree_index).unwrap()].0))]
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    #[account(
        mut,
        address = anchor_lang::prelude::Pubkey::find_program_address(&[merkle_tree.key().to_bytes().as_ref()], &MerkleTreeProgram::id()).0
    )]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub sender: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient_fee: UncheckedAccount<'info>,
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut)]
    pub relayer_recipient: AccountInfo<'info>,
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut)]
    pub escrow: AccountInfo<'info>,

}

// split into two tx
// tx checks which data it has and computes accordingly
// tx checks if other compute was already completed
// if yes insert leaves etc

pub fn process_shielded_transfer_2_inputs<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info,ShieldedTransfer2Inputs<'info>>,
    proof: [u8; 256],
    merkle_root: [u8; 32],
    public_amount: [u8; 32],
    ext_data_hash: [u8; 32],
    nullifier0: [u8; 32],
    nullifier1: [u8; 32],
    leaf_right: [u8; 32],
    leaf_left: [u8; 32],
    ext_amount: i64,
    fee_amount: [u8; 32],
    mint_pubkey: [u8;32],
    encrypted_utxos: Vec<u8>,
    merkle_tree_index: u64,
    relayer_fee: u64,
) -> Result<()> {

    // trait with the nunber of inputs and commitments
    // Put nullifier accounts in remaining accounts
    // Put commitment accounts in the remaining accounts
    // make the instruction flexible enough such that I can easily call it in a second tx
    // actually with that I can easily implement it in 2 tx in the first place

    let mut tx = LightTransaction::new(
        proof,
        merkle_root,
        public_amount,
        ext_data_hash,
        fee_amount,
        mint_pubkey,
        Vec::<Vec<u8>>::new(), // checked_public_inputs
        vec![nullifier0.to_vec(), nullifier1.to_vec()],
        vec![(leaf_left.to_vec(), leaf_right.to_vec())],
        encrypted_utxos,
        merkle_tree_index,
        ext_amount,
        relayer_fee,
        ctx
    );
    tx.verify()?;
    tx.check_tx_integrity_hash()?;
    tx.check_root()?;
    sol_log_compute_units();
    msg!("leaves");
    tx.insert_leaves()?;
    sol_log_compute_units();
    msg!("nullifiers");
    tx.insert_nullifiers()?;
    sol_log_compute_units();
    tx.transfer_user_funds()?;
    tx.transfer_fee()?;
    tx.check_completion()?;
    Ok(())
}
