use anchor_lang::prelude::*;

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
    /// CHECK: Check that mint authority is derived from signer
    // #[account(mut, seeds = [b"authority", authority.key().to_bytes().as_slice(), mint.key().to_bytes().as_slice()], bump,)]
    pub authority_pda: UncheckedAccount<'info>,
    /// CHECK: this account
    #[account(mut)]
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK: this account
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    pub compressed_pda_program: UncheckedAccount<'info>, // Program<'info, psp_compressed_pda::program::CompressedPda>,
    /// CHECK: this account in psp account compression program
    #[account(mut)]
    pub psp_account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    pub account_compression_program: UncheckedAccount<'info>,
}

// TODO: parse utxos a more efficient way, since owner is sent multiple times this way
#[derive(Debug)]
#[account]
pub struct InstructionDataTransfer {
    proof_a: [u8; 32],
    proof_b: [u8; 64],
    proof_c: [u8; 32],
    in_utxos: Vec<TokenInUtxo>,
    low_element_indexes: Vec<u16>,
    root_indexes: Vec<u64>,
    rpc_fee: Option<u64>,
    out_utxo: Vec<TokenOutUtxo>,
}
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
