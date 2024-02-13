use anchor_lang::{
    prelude::*,
    solana_program::{
        keccak::{hash, hashv},
        pubkey::Pubkey,
    },
};
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::{Hasher, Poseidon};

declare_id!("6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ");

#[program]
pub mod psp_compressed_pda {
    use super::*;

    /// This function can be used to transfer sol and execute any other compressed transaction.
    pub fn execute_compressed_transaction(
        _ctx: Context<TransferInstruction>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let _inputs: InstructionDataTransfer = InstructionDataTransfer::try_deserialize_unchecked(
            &mut [vec![0u8; 8], inputs].concat().as_slice(),
        )?;
        Ok(())
    }

    // TODO: add compress and decompress sol as a wrapper around process_execute_compressed_transaction

    // TODO: add create_pda as a wrapper around process_execute_compressed_transaction
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
    pub cpi_signature_account: Option<Account<'info, CpiSignatureAccount>>,
}

#[account]
pub struct CpiSignatureAccount {
    pub signatures: Vec<CpiSignature>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CpiSignature {
    pub program: Pubkey,
    pub tlv_hash: [u8; 32],
    pub tlv_data: TlvDataElement,
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
    out_utxo: SerializedUtxos,
}

// there are two sources I can get the pubkey from the transaction object and the other account keys
// the index starts with the accounts keys of the transaction object, if the index is larger than the length of the accounts keys
// we access the pubkey array with our additiona pubkeys

// we need a general macro that just derives a serializable struct from a struct that replaces every pubkey with a u8
// the struct should be borsh serializable and deserializable
// additionally the macro should derive a function that converts the serializable struct back to the original struct with the additional input
// of a slice of account infos where it gets the pubkeys from
// additionally the macro needs to derive a function that converts the original struct into the serializable struct and outputs the struct and the pubkeys
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SerializedUtxos {
    pub pubkey_array: Vec<Pubkey>,
    pub in_utxo: Vec<InUtxoSerializable>,
    pub out_utxo: Vec<OutUtxoSerializable>,
}

impl SerializedUtxos {
    pub fn in_utxos_from_serialized_utxos(
        &self,
        accounts: &[Pubkey],
        merkle_tree_accounts: &[Pubkey],
    ) -> Vec<Utxo> {
        let mut in_utxos = Vec::new();
        for (i, in_utxo) in self.in_utxo.iter().enumerate() {
            let owner = if (in_utxo.owner as usize) < accounts.len() {
                accounts[in_utxo.owner as usize]
            } else {
                self.pubkey_array[in_utxo.owner.saturating_sub(accounts.len() as u8) as usize]
            };
            let lamports = in_utxo.lamports;
            let data = in_utxo.data.as_ref().map(|data| {
                data.tlv_from_serializable_tlv(
                    [accounts, self.pubkey_array.as_slice()].concat().as_slice(),
                )
            });
            let mut utxo = Utxo {
                owner,
                blinding: [0u8; 32],
                lamports,
                data,
            };
            utxo.update_blinding(merkle_tree_accounts[i].key(), in_utxo.leaf_index as usize)
                .unwrap();
            in_utxos.push(utxo);
        }
        in_utxos
    }

    pub fn out_utxos_from_serialized_utxos(
        &self,
        accounts: &[Pubkey],
        merkle_tree_accounts: &[Pubkey],
        leaf_indices: &[u16],
    ) -> Vec<Utxo> {
        let mut out_utxos = Vec::new();
        for (i, in_utxo) in self.in_utxo.iter().enumerate() {
            let owner = if (in_utxo.owner as usize) < accounts.len() {
                accounts[in_utxo.owner as usize]
            } else {
                self.pubkey_array[in_utxo.owner.saturating_sub(accounts.len() as u8) as usize]
            };
            let lamports = in_utxo.lamports;
            let data = in_utxo.data.as_ref().map(|data| {
                data.tlv_from_serializable_tlv(
                    [accounts, self.pubkey_array.as_slice()].concat().as_slice(),
                )
            });
            let mut utxo = Utxo {
                owner,
                blinding: [0u8; 32],
                lamports,
                data,
            };
            utxo.update_blinding(merkle_tree_accounts[i].key(), leaf_indices[i] as usize)
                .unwrap();
            out_utxos.push(utxo);
        }
        out_utxos
    }
}

#[derive(Debug)]
#[account]
pub struct InUtxoSerializable {
    pub owner: u8,
    pub leaf_index: u32,
    pub lamports: u64,
    pub data: Option<TlvSerializable>,
}

// no need to send blinding is computed onchain
#[derive(Debug)]
#[account]
pub struct OutUtxoSerializable {
    pub owner: u8,
    pub lamports: u64,
    pub data: Option<TlvSerializable>,
}

// blinding we just need to send the leafIndex
#[derive(Debug)]
#[account]
pub struct Utxo {
    pub owner: Pubkey,
    pub blinding: [u8; 32],
    pub lamports: u64,
    pub data: Option<Tlv>,
}

impl Utxo {
    pub fn update_blinding(&mut self, merkle_tree_pda: Pubkey, index_of_leaf: usize) -> Result<()> {
        self.blinding = Poseidon::hashv(&[
            &hash(merkle_tree_pda.to_bytes().as_slice()).to_bytes()[0..30],
            index_of_leaf.to_le_bytes().as_slice(),
        ])
        .unwrap();
        Ok(())
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TlvSerializable {
    pub tlv_elements: Vec<TlvDataElementSerializable>,
}

impl TlvSerializable {
    pub fn tlv_from_serializable_tlv(&self, accounts: &[Pubkey]) -> Tlv {
        let mut tlv_elements = Vec::new();
        for tlv_element in &self.tlv_elements {
            let owner = accounts[tlv_element.owner as usize];
            tlv_elements.push(TlvDataElement {
                discriminator: tlv_element.discriminator,
                owner,
                data: tlv_element.data.clone(),
            });
        }
        Tlv { tlv_elements }
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct Tlv {
    pub tlv_elements: Vec<TlvDataElement>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TlvDataElementSerializable {
    pub discriminator: [u8; 8],
    pub owner: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TlvDataElement {
    pub discriminator: [u8; 8],
    pub owner: Pubkey,
    pub data: Vec<u8>,
}

// /// Time lock escrow example:
// /// escrow tlv data -> compressed token program
// /// let escrow_data = {
// ///   owner: Pubkey, // owner is the user pubkey
// ///   release_slot: u64,
// ///   deposit_slot: u64,
// /// };
// ///
// /// let escrow_tlv_data = TlvDataElement {
// ///   discriminator: [1,0,0,0,0,0,0,0],
// ///   owner: escrow_program_id,
// ///   data: escrow_data,
// ///   tlv_data: Some(token_tlv.try_to_vec()?),
// /// };
// /// let token_tlv = TlvDataElement {
// ///   discriminator: [2,0,0,0,0,0,0,0],
// ///   owner: token_program,
// ///   data: token_data,
// ///   tlv_data: None,
// /// };
// /// let token_data = Account {
// ///  mint,
// ///  owner,
// ///  amount: 10_000_000u64,
// ///  delegate: None,
// ///  state: Initialized, (u64)
// ///  is_native: None,
// ///  delegated_amount: 0u64,
// ///  close_authority: None,
// /// };
// ///
// #[derive(Debug, Clone)]
// pub struct TlvDataElement {
//     pub discriminator: [u8; 8],
//     pub owner: Pubkey,
//     pub data: Vec<u8>,
//     pub tlv_data: Option<Box<TlvDataElement>>,
// }

// impl BorshSerialize for TlvDataElement {
//     fn serialize<W: std::io::Write>(
//         &self,
//         writer: &mut W,
//     ) -> std::result::Result<(), std::io::Error> {
//         self.discriminator.serialize(writer)?;
//         self.owner.serialize(writer)?;
//         self.data.serialize(writer)?;
//         match &self.tlv_data {
//             Some(boxed) => {
//                 1u8.serialize(writer)?; // Indicate that `tlv_data` is present
//                 boxed.serialize(writer)?;
//             }
//             None => {
//                 0u8.serialize(writer)?; // Indicate that `tlv_data` is not present
//             }
//         }
//         Ok(())
//     }
// }

// impl BorshDeserialize for TlvDataElement {
//     fn deserialize(buf: &mut &[u8]) -> std::result::Result<Self, std::io::Error> {
//         let discriminator = <[u8; 8]>::deserialize(buf)?;
//         let owner = <Pubkey>::deserialize(buf)?;
//         let data = Vec::<u8>::deserialize(buf)?;
//         let tlv_data_indicator: u8 = BorshDeserialize::deserialize(buf)?;
//         let tlv_data = if tlv_data_indicator == 0 {
//             None
//         } else {
//             Some(Box::new(TlvDataElement::deserialize(buf)?))
//         };

//         Ok(TlvDataElement {
//             discriminator,
//             owner,
//             data,
//             tlv_data,
//         })
//     }

//     fn deserialize_reader<R: std::io::Read>(
//         reader: &mut R,
//     ) -> std::result::Result<Self, std::io::Error> {
//         let mut discriminator = [0u8; 8];
//         reader.read_exact(&mut discriminator)?;

//         let mut owner = [0u8; 32];
//         reader.read_exact(&mut owner)?;

//         // Directly read the length of the data vector from the reader
//         let mut data_len_bytes = [0u8; 4];
//         reader.read_exact(&mut data_len_bytes)?;
//         let data_len = u32::from_le_bytes(data_len_bytes); // Assumes little endian. Adjust if necessary.

//         let mut data = vec![0u8; data_len as usize];
//         reader.read_exact(&mut data)?;

//         // Directly read the tlv_data_indicator from the reader
//         let mut tlv_data_indicator_bytes = [0u8; 1];
//         reader.read_exact(&mut tlv_data_indicator_bytes)?;
//         let tlv_data_indicator = tlv_data_indicator_bytes[0];

//         let tlv_data = if tlv_data_indicator == 0 {
//             None
//         } else {
//             Some(Box::new(TlvDataElement::deserialize_reader(reader)?))
//         };

//         Ok(TlvDataElement {
//             discriminator,
//             owner: Pubkey::new_from_array(owner),
//             data,
//             tlv_data,
//         })
//     }
// }
