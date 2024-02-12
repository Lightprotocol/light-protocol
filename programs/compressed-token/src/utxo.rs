use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug)]
#[account]
pub struct TokenInUtxo {
    pub owner: Pubkey,
    pub blinding: [u8; 32],
    pub lamports: u64,
    pub data: Option<TlvData>,
}

#[derive(Debug)]
#[account]
pub struct TokenOutUtxo {
    pub owner: Pubkey,
    pub blinding: [u8; 32],
    pub lamports: u64,
    pub data: Option<TlvData>,
}

/// Time lock escrow example:
/// escrow tlv data -> compressed token program
/// let escrow_data = {
///   owner: Pubkey, // owner is the user pubkey
///   release_slot: u64,
///   deposit_slot: u64,
/// };
///
/// let escrow_tlv_data = TlvData {
///   discriminator: [1,0,0,0,0,0,0,0],
///   owner: escrow_program_id,
///   data: escrow_data,
///   tlv_data: Some(token_tlv.try_to_vec()?),
/// };
/// let token_tlv = TlvData {
///   discriminator: [2,0,0,0,0,0,0,0],
///   owner: token_program,
///   data: token_data,
///   tlv_data: None,
/// };
/// let token_data = TokenAccount {
///  mint,
///  owner,
///  amount: 10_000_000u64,
///  delegate: None,
///  state: Initialized, (u64)
///  is_native: None,
///  delegated_amount: 0u64,
///  close_authority: None,
/// };
///
#[derive(Debug, Clone)]
pub struct TlvData {
    pub discriminator: [u8; 8],
    pub owner: Pubkey,
    pub data: Vec<u8>,
    pub tlv_data: Option<Box<TlvData>>,
}

impl BorshSerialize for TlvData {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::result::Result<(), std::io::Error> {
        self.discriminator.serialize(writer)?;
        self.owner.serialize(writer)?;
        self.data.serialize(writer)?;
        match &self.tlv_data {
            Some(boxed) => {
                1u8.serialize(writer)?; // Indicate that `tlv_data` is present
                boxed.serialize(writer)?;
            }
            None => {
                0u8.serialize(writer)?; // Indicate that `tlv_data` is not present
            }
        }
        Ok(())
    }
}

impl BorshDeserialize for TlvData {
    fn deserialize(buf: &mut &[u8]) -> std::result::Result<Self, std::io::Error> {
        let discriminator = <[u8; 8]>::deserialize(buf)?;
        let owner = <Pubkey>::deserialize(buf)?;
        let data = Vec::<u8>::deserialize(buf)?;
        let tlv_data_indicator: u8 = BorshDeserialize::deserialize(buf)?;
        let tlv_data = if tlv_data_indicator == 0 {
            None
        } else {
            Some(Box::new(TlvData::deserialize(buf)?))
        };

        Ok(TlvData {
            discriminator,
            owner,
            data,
            tlv_data,
        })
    }

    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> std::result::Result<Self, std::io::Error> {
        let mut discriminator = [0u8; 8];
        reader.read_exact(&mut discriminator)?;

        let mut owner = [0u8; 32];
        reader.read_exact(&mut owner)?;

        // Directly read the length of the data vector from the reader
        let mut data_len_bytes = [0u8; 4];
        reader.read_exact(&mut data_len_bytes)?;
        let data_len = u32::from_le_bytes(data_len_bytes); // Assumes little endian. Adjust if necessary.

        let mut data = vec![0u8; data_len as usize];
        reader.read_exact(&mut data)?;

        // Directly read the tlv_data_indicator from the reader
        let mut tlv_data_indicator_bytes = [0u8; 1];
        reader.read_exact(&mut tlv_data_indicator_bytes)?;
        let tlv_data_indicator = tlv_data_indicator_bytes[0];

        let tlv_data = if tlv_data_indicator == 0 {
            None
        } else {
            Some(Box::new(TlvData::deserialize_reader(reader)?))
        };

        Ok(TlvData {
            discriminator,
            owner: Pubkey::new_from_array(owner),
            data,
            tlv_data,
        })
    }
}
