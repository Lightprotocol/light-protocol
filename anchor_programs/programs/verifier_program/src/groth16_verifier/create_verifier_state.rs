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
    merkle_root:          [u8;32],
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
        merkle_root: [u8; 32],
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
    let verifier_state_data = &mut match ctx.accounts.verifier_state.load_mut() {
            Ok(res) => {
                    if res.signing_address == ctx.accounts.signing_address.key() {
                        Ok(res)
                    } else {
                        err!(ErrorCode::WrongSigner)
                    }
                },
            Err(_)  => ctx.accounts.verifier_state.load_init()
    }?;

    verifier_state_data.signing_address = ctx.accounts.signing_address.key();
    verifier_state_data.merkle_root = merkle_root.clone();
    verifier_state_data.amount = amount.clone();
    verifier_state_data.merkle_tree_index = merkle_tree_index[0].clone();
    verifier_state_data.relayer_fee = u64::from_le_bytes(relayer_fee.try_into().unwrap()).clone();
    verifier_state_data.recipient = Pubkey::new(&recipient).clone();
    verifier_state_data.tx_integrity_hash = tx_integrity_hash.clone();
    verifier_state_data.ext_amount = ext_amount.clone();
    verifier_state_data.fee = relayer_fee.clone();//tx_fee.clone();
    verifier_state_data.leaf_left = leaf_left;
    verifier_state_data.leaf_right = leaf_right;
    verifier_state_data.nullifier0 = nullifier0;
    verifier_state_data.nullifier1 = nullifier1;
    verifier_state_data.encrypted_utxos = encrypted_utxos[..222].try_into().unwrap();

    // initing pairs to prepared inputs
    init_pairs_instruction(verifier_state_data)?;
    _process_instruction(41, verifier_state_data, verifier_state_data.current_index as usize)?;
    verifier_state_data.current_index = 1;
    verifier_state_data.current_instruction_index = 1;
    verifier_state_data.computing_prepared_inputs = true;

    // miller loop
    verifier_state_data.proof_a_bytes = proof[0..64].try_into().unwrap();
    verifier_state_data.proof_b_bytes = proof[64..64 + 128].try_into().unwrap();
    verifier_state_data.proof_c_bytes = proof[64 + 128..256].try_into().unwrap();
    verifier_state_data.ml_max_compute = 1_350_000;
    verifier_state_data.f_bytes[0] = 1;
    let proof_b = parse_proof_b_from_bytes(&verifier_state_data.proof_b_bytes.to_vec());

    verifier_state_data.r_bytes = parse_r_to_bytes(G2HomProjective {
        x: proof_b.x,
        y: proof_b.y,
        z: Fp2::one(),
    });


    check_tx_integrity_hash(
        verifier_state_data.recipient.to_bytes().to_vec(),
        verifier_state_data.ext_amount.to_vec(),
        verifier_state_data.signing_address.key().to_bytes().to_vec(),
        verifier_state_data.fee.to_vec(),
        verifier_state_data.tx_integrity_hash.to_vec(),
        verifier_state_data.merkle_tree_index,
        verifier_state_data.encrypted_utxos[..222].to_vec(),
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
