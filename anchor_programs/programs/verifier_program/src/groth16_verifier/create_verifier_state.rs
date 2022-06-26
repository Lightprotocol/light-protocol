use anchor_lang::prelude::*;
use ark_ec::bn::g2::G2HomProjective;
use ark_ff::Fp2;
use ark_std::One;
use crate::errors::ErrorCode;
use crate::groth16_verifier::VerifierState;
use crate::groth16_verifier::init_pairs_instruction;
use crate::groth16_verifier::_process_instruction;
use crate::groth16_verifier::parse_proof_b_from_bytes;
use crate::groth16_verifier::parse_r_to_bytes;
use ark_ed_on_bn254::Fq;
use ark_ff::PrimeField;

#[derive(Accounts)]
#[instruction(
    proof:              [u8;256],
    root_hash:          [u8;32],
    amount:             [u8;32],
    tx_integrity_hash:  [u8;32]
)]
pub struct CreateVerifierState<'info> {
    #[account(init_if_needed, seeds = [tx_integrity_hash.as_ref(), b"storage"], bump,  payer=signing_address, space= 5 * 1024 as usize)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    /// First time therefore the signing address is not checked but saved to be checked in future instructions.
    /// Is checked in the tx integrity hash
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn process_create_verifier_state(
        ctx: Context<CreateVerifierState>,
        proof: [u8; 256],
        root_hash: [u8; 32],
        amount: [u8; 32],
        tx_integrity_hash: [u8; 32],
        nullifier0: [u8; 32],
        nullifier1: [u8; 32],
        leaf_right: [u8; 32],
        leaf_left: [u8; 32],
        recipient: [u8; 32],
        ext_amount: [u8; 8],
        _relayer: [u8; 32],
        relayer_fee: [u8; 8],
        encrypted_utxos: [u8; 256],
        merkle_tree_index: [u8; 1]
    ) -> Result<()> {
    // if not initialized this will run load_init
    let tmp_account = &mut match ctx.accounts.verifier_state.load_mut() {
            Ok(res) => res,
            Err(_)  => ctx.accounts.verifier_state.load_init()?
    };

    tmp_account.signing_address = ctx.accounts.signing_address.key();
    tmp_account.root_hash = root_hash.clone();
    tmp_account.amount = amount.clone();
    tmp_account.merkle_tree_index = merkle_tree_index[0].clone();
    tmp_account.relayer_fee = u64::from_le_bytes(relayer_fee.try_into().unwrap()).clone();
    tmp_account.recipient = Pubkey::new(&recipient).clone();
    tmp_account.tx_integrity_hash = tx_integrity_hash.clone();
    tmp_account.ext_amount = ext_amount.clone();
    tmp_account.fee = relayer_fee.clone();//tx_fee.clone();
    tmp_account.leaf_left = leaf_left;
    tmp_account.leaf_right = leaf_right;
    tmp_account.nullifier0 = nullifier0;
    tmp_account.nullifier1 = nullifier1;
    tmp_account.encrypted_utxos = encrypted_utxos[..222].try_into().unwrap();

    // initing pairs to prepared inputs
    init_pairs_instruction(tmp_account)?;
    _process_instruction(41, tmp_account, tmp_account.current_index as usize)?;
    tmp_account.current_index = 1;
    tmp_account.current_instruction_index = 1;
    tmp_account.computing_prepared_inputs = true;

    // miller loop
    tmp_account.proof_a_bytes = proof[0..64].try_into().unwrap();
    tmp_account.proof_b_bytes = proof[64..64 + 128].try_into().unwrap();
    tmp_account.proof_c_bytes = proof[64 + 128..256].try_into().unwrap();
    tmp_account.ml_max_compute = 1_350_000;
    tmp_account.f_bytes[0] = 1;
    let proof_b = parse_proof_b_from_bytes(&tmp_account.proof_b_bytes.to_vec());

    tmp_account.r_bytes = parse_r_to_bytes(G2HomProjective {
        x: proof_b.x,
        y: proof_b.y,
        z: Fp2::one(),
    });


    check_tx_integrity_hash(
        recipient.to_vec(),
        ext_amount.to_vec(),
        ctx.accounts.signing_address.key().to_bytes().to_vec(),
        relayer_fee.to_vec(),
        tx_integrity_hash.to_vec(),
        merkle_tree_index[0],
        encrypted_utxos[..222].to_vec(),
        merkle_tree_program::utils::config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_index[0] as usize].0.to_vec(),
    )
}

fn check_tx_integrity_hash(
    recipient: Vec<u8>,
    ext_amount: Vec<u8>,
    relayer: Vec<u8>,
    fee: Vec<u8>,
    tx_integrity_hash: Vec<u8>,
    merkle_tree_index: u8,
    encrypted_utxos: Vec<u8>,
    merkle_tree_pda_pubkey: Vec<u8>,
) -> Result<()> {
    let input = [
        recipient,
        ext_amount,
        relayer,
        fee,
        merkle_tree_pda_pubkey,
        vec![merkle_tree_index],
        encrypted_utxos,
    ]
    .concat();
    msg!("integrity_hash inputs: {:?}", input);
    let hash = solana_program::keccak::hash(&input[..]).try_to_vec()?;
    msg!("hash computed {:?}", hash);

    if Fq::from_be_bytes_mod_order(&hash[..]) != Fq::from_le_bytes_mod_order(&tx_integrity_hash) {
        msg!("tx_integrity_hash verification failed.{:?} != {:?}", &hash[..] , &tx_integrity_hash);
        return err!(ErrorCode::WrongTxIntegrityHash);
    }
    Ok(())
}
