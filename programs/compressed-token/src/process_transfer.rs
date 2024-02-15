use anchor_lang::prelude::*;
use light_hasher::{errors::HasherError, DataHasher, Hasher, Poseidon};
use light_utils::hash_to_bn254_field_size_le;
use psp_compressed_pda::SerializedUtxos;
// use light_verifier_sdk::light_transaction::ProofCompressed;

// use crate::utxo::{TokenInUtxo, TokenOutUtxo};

// #[account]
// #[derive(Debug, PartialEq, Eq, Default)]
// #[allow(non_camel_case_types)]
// pub struct u256 {
//     pub data: [u8; 32],
// }

// #[account]
// #[derive(Debug, PartialEq, Eq)]
// pub struct TransferOutputUtxo {
//     pub owner: u256,
//     pub amounts: [u64; 2],
//     pub spl_asset_mint: Option<Pubkey>,
//     pub meta_hash: Option<u256>,
//     pub address: Option<u256>,
// }

// pub fn from_transfer_output_utxo(utxo: TransferOutputUtxo) -> Utxo {
//     // beet big number deserialiazation is little endian
//     let mut owner = utxo.owner.data;
//     owner.reverse();
//     Utxo {
//         version: 0,
//         pool_type: 0,
//         amounts: utxo.amounts,
//         spl_asset_mint: Some(utxo.spl_asset_mint.unwrap_or_default()),
//         owner,
//         blinding: [0u8; 32],
//         data_hash: [0u8; 32],
//         meta_hash: utxo.meta_hash.unwrap_or(u256 { data: [0u8; 32] }).data,
//         address: utxo.address.unwrap_or(u256 { data: [0u8; 32] }).data,
//         message: None,
//     }
// }

pub fn process_transfer<'a, 'b, 'c, 'info: 'b + 'c>(
    _ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    let _inputs: InstructionDataTransfer = InstructionDataTransfer::try_deserialize_unchecked(
        &mut [vec![0u8; 8], inputs].concat().as_slice(),
    )?;
    // msg!("in_utxo_hashes {:?}", inputs.in_utxo_hashes);
    // // TODO: refactor into generic function to reuse for input validation in 8in2out
    // if inputs.low_element_indexes.len() > 2
    //     && inputs.low_element_indexes.len() != inputs.in_utxo_hashes.len()
    // {
    //     msg!("number of low element indexes invalid {} > 2 or not equal to number of in utxo hashes {} != {}", inputs.low_element_indexes.len(),inputs.low_element_indexes.len(),  inputs.in_utxo_hashes.len());
    //     panic!();
    // }
    // if inputs.out_utxo.len() > 2 {
    //     msg!("number of out_utxo invalid {} > 2", inputs.out_utxo.len());
    //     panic!();
    // }
    // if inputs.in_utxo_hashes.len() > 2 {
    //     msg!(
    //         "number of in_utxo_hashes invalid {} > 2",
    //         inputs.in_utxo_hashes.len()
    //     );
    //     panic!();
    // }

    // let proof = ProofCompressed {
    //     a: inputs.proof_a,
    //     b: inputs.proof_b,
    //     c: inputs.proof_c,
    // };

    // let mut out_utxos: Vec<Utxo> = Vec::new();
    // let mut merkle_root_indexes = [0usize; 2];
    // for (i, utxo) in inputs.out_utxo.iter().enumerate() {
    //     if utxo.is_some() {
    //         let utxo = utxo.as_ref().unwrap();
    //         // TODO: optimize vec usage
    //         let deserialized_utxo: TransferOutputUtxo =
    //             TransferOutputUtxo::try_deserialize_unchecked(
    //                 &mut [vec![0u8; 8], utxo.to_vec()].concat().as_slice(),
    //             )
    //             .unwrap();
    //         out_utxos.push(from_transfer_output_utxo(deserialized_utxo));
    //         merkle_root_indexes[i] = inputs.root_indexes[i].unwrap() as usize;
    //     }
    // }

    // let public_amount = Amounts {
    //     sol: inputs.public_amount_sol,
    //     spl: inputs.public_amount_spl,
    // };
    // // let mut low_element_indexes = [0u16; 2];
    // // for (i, index) in inputs.low_element_indexes.iter().enumerate() {
    // //     low_element_indexes[i] = *index;
    // // }
    // let input = PublicTransactionInput {
    //     ctx: &ctx,
    //     message: None,
    //     proof: &proof,
    //     public_amount: Some(&public_amount),
    //     in_utxo_hashes: &inputs.in_utxo_hashes,
    //     in_utxo_data_hashes: [None, None],
    //     out_utxos: out_utxos.clone(),
    //     merkle_root_indexes,
    //     rpc_fee: inputs.rpc_fee,
    //     pool_type: &[0u8; 32],
    //     verifyingkey: &VERIFYINGKEY_PUBLIC_PROGRAM_TRANSACTION2_IN2_OUT_MAIN,
    //     program_id: None,
    //     new_addresses: &[None, None],
    //     transaction_hash: None,
    //     low_element_indexes: &inputs.low_element_indexes,
    // };
    // let mut transaction = PublicTransaction::<
    //     0,
    //     2,
    //     2,
    //     14,
    //     TransferInstruction<'info>,
    //     PublicTransactionPublicInputs<2, 2>,
    // >::new(input);

    // // this is only for testing
    // #[cfg(not(target_os = "solana"))]
    // {
    //     transaction.tx_integrity_hash = [0u8; 32];
    //     transaction.state_merkle_roots = test_state_roots.unwrap();
    //     transaction.out_utxo_hashes =
    //         vec![out_utxos[0].hash().unwrap(), out_utxos[1].hash().unwrap()];
    //     transaction.mint_pubkey = [
    //         0, 24, 59, 207, 17, 191, 51, 84, 25, 96, 177, 164, 233, 142, 128, 208, 115, 82, 0, 223,
    //         237, 121, 0, 231, 241, 213, 140, 224, 58, 185, 152, 253,
    //     ];
    //     transaction.verify()?;
    // }

    // #[cfg(target_os = "solana")]
    // transaction.transact()?;
    Ok(())
}

/// These are the base accounts additionally Merkle tree and queue accounts are required.
/// These additional accounts are passed as remaining accounts.
/// 1 Merkle tree for each in utxo one queue and Merkle tree account each for each out utxo.
#[derive(Accounts)]
pub struct TransferInstruction<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// Check that mint authority is derived from signer
    // #[account(mut, seeds = [b"authority", authority.key().to_bytes().as_slice(), mint.key().to_bytes().as_slice()], bump,)]
    pub authority_pda: UncheckedAccount<'info>,
    /// CHECK this account
    #[account(mut)]
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK this account
    pub noop_program: UncheckedAccount<'info>,
    pub compressed_pda_program: UncheckedAccount<'info>, // Program<'info, psp_compressed_pda::program::CompressedPda>,
    /// CHECK this account in psp account compression program
    #[account(mut)]
    pub psp_account_compression_authority: UncheckedAccount<'info>,
    /// CHECK this account in psp account compression program
    pub account_compression_program: UncheckedAccount<'info>,
}

// TODO: parse utxos a more efficient way, since owner is sent multiple times this way
#[derive(Debug)]
#[account]
pub struct InstructionDataTransfer {
    proof_a: [u8; 32],
    proof_b: [u8; 64],
    proof_c: [u8; 32],
    low_element_indexes: Vec<u16>,
    root_indexes: Vec<u64>,
    rpc_fee: Option<u64>,
    serialized_utxos: SerializedUtxos,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum AccountState {
    Uninitialized,
    Initialized,
    Frozen,
}

#[derive(Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct TokenTlvData {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: AccountState,
    /// If is_some, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    pub is_native: Option<u64>,
    /// The amount delegated
    pub delegated_amount: u64,
    /// Optional authority to close the account.
    pub close_authority: Option<Pubkey>,
}

impl DataHasher for TokenTlvData {
    fn hash(&self) -> std::result::Result<[u8; 32], HasherError> {
        let delegate = match self.delegate {
            Some(delegate) => {
                hash_to_bn254_field_size_le(delegate.to_bytes().as_slice())
                    .unwrap()
                    .0
            }
            None => [0u8; 32],
        };
        let close_authority = match self.close_authority {
            Some(close_authority) => {
                hash_to_bn254_field_size_le(close_authority.to_bytes().as_slice())
                    .unwrap()
                    .0
            }
            None => [0u8; 32],
        };

        Poseidon::hashv(&[
            &hash_to_bn254_field_size_le(self.mint.to_bytes().as_slice())
                .unwrap()
                .0,
            &hash_to_bn254_field_size_le(self.owner.to_bytes().as_slice())
                .unwrap()
                .0,
            &self.amount.to_le_bytes(),
            &delegate,
            &(self.state as u8).to_le_bytes(),
            &self.is_native.unwrap_or_default().to_le_bytes(),
            &self.delegated_amount.to_le_bytes(),
            &close_authority,
        ])
    }
}

#[cfg(test)]
mod test {
    use psp_compressed_pda::{OutUtxo, SerializedUtxos, Tlv, TlvDataElement, Utxo};

    use super::*;

    #[test]
    fn test_add_utxos_with_token_tlv_data() {
        let mut serialized_utxos = SerializedUtxos {
            pubkey_array: vec![],
            u64_array: vec![],
            in_utxos: vec![],
            out_utxos: vec![],
        };

        let token_program = Pubkey::new_unique();
        let merkle_tree_pda = Pubkey::new_unique();
        let owner_pubkey = Pubkey::new_unique();
        let mint_pubkey = Pubkey::new_unique();
        let delegate_pubkey = Pubkey::new_unique(); // Assuming there's a delegate for this example
        let accounts = vec![owner_pubkey, mint_pubkey, token_program];

        // Creating TokenTlvData
        let token_tlv_data = TokenTlvData {
            mint: mint_pubkey,
            owner: owner_pubkey,
            amount: 10_000_000,
            delegate: Some(delegate_pubkey),
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 5000,
            close_authority: None,
        };

        // Assuming we have a way to serialize TokenTlvData to Vec<u8>
        let token_data_serialized = token_tlv_data.try_to_vec().unwrap();
        let token_data_hash = token_tlv_data.hash().unwrap(); // Assuming hash() returns a Result

        // Creating TLV data element with TokenTlvData
        let token_tlv_data_element = TlvDataElement {
            discriminator: [2; 8],
            owner: token_program,
            data: token_data_serialized,
            data_hash: token_data_hash,
        };

        let tlv_data = Tlv {
            tlv_elements: vec![token_tlv_data_element],
        };

        // Convert TLV data to a serializable format
        let mut pubkey_array_for_tlv = Vec::new();
        let tlv_serializable = tlv_data.to_serializable_tlv(&mut pubkey_array_for_tlv, &accounts);

        let mut utxo = Utxo {
            owner: owner_pubkey,
            blinding: [0u8; 32],
            lamports: 100,
            data: Some(tlv_data.clone()),
        };
        let leaf_index = 1u32;
        utxo.update_blinding(merkle_tree_pda, (leaf_index as usize).clone())
            .unwrap();

        // Assuming add_in_utxos is modified to accept UTXOs with TLV data correctly
        serialized_utxos
            .add_in_utxos(&[utxo.clone()], &accounts, &[leaf_index])
            .unwrap();

        // Create OutUtxo
        let out_utxo = OutUtxo {
            owner: owner_pubkey,
            lamports: 100,
            data: Some(tlv_data),
        };

        // Add OutUtxo
        serialized_utxos
            .add_out_utxos(&[out_utxo], &accounts)
            .unwrap();

        assert_eq!(
            serialized_utxos.in_utxos.len(),
            1,
            "Should have added one UTXO with TLV data"
        );
        assert!(
            serialized_utxos.in_utxos[0].data.is_some(),
            "UTXO should contain TLV data"
        );
        assert_eq!(
            serialized_utxos.out_utxos.len(),
            1,
            "Should have added one out UTXO with TLV data"
        );
        assert!(
            serialized_utxos.out_utxos[0].data.is_some(),
            "UTXO should contain TLV data"
        );
        // Verify that TLV data was serialized correctly
        let serialized_tlv_data = serialized_utxos.in_utxos[0].data.as_ref().unwrap();
        assert_eq!(
            *serialized_tlv_data, tlv_serializable,
            "Serialized TLV data should match the expected serialized version"
        );
        let deserialized_in_utxos = serialized_utxos
            .in_utxos_from_serialized_utxos(&accounts, &[merkle_tree_pda])
            .unwrap();
        assert_eq!(deserialized_in_utxos[0], utxo);

        let deserialized_out_utxos = serialized_utxos
            .out_utxos_from_serialized_utxos(&accounts, &[merkle_tree_pda], &[1u32])
            .unwrap();
        assert_eq!(deserialized_out_utxos[0], utxo);
    }
}
