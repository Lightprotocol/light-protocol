use anchor_lang::{prelude::*, AnchorDeserialize, AnchorSerialize};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError, slice::ZeroCopySliceBorsh};
use zerocopy::{little_endian::U64, Ref};

#[derive(Debug, Default, Clone, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct BatchCompressInstructionDataBorsh {
    pub pubkeys: Vec<Pubkey>,
    pub amounts: Vec<u64>,
    pub lamports: Option<u64>,
}

pub struct BatchCompressInstructionData<'a> {
    pub pubkeys: ZeroCopySliceBorsh<'a, light_utils::pubkey::Pubkey>,
    pub amounts: ZeroCopySliceBorsh<'a, U64>,
    pub lamports: Option<Ref<&'a [u8], U64>>,
}

impl<'a> Deserialize<'a> for BatchCompressInstructionData<'a> {
    type Output = Self;

    fn zero_copy_at(bytes: &'a [u8]) -> std::result::Result<(Self, &'a [u8]), ZeroCopyError> {
        let (pubkeys, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
        let (amounts, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
        let (lamports, bytes) = Option::<U64>::zero_copy_at(bytes)?;
        Ok((
            Self {
                pubkeys,
                amounts,
                lamports,
            },
            bytes,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_batch_compress_instruction_data() {
        let data = super::BatchCompressInstructionDataBorsh {
            pubkeys: vec![Pubkey::new_unique(), Pubkey::new_unique()],
            amounts: vec![1, 2],
            lamports: Some(3),
        };
        let mut vec = Vec::new();
        data.serialize(&mut vec).unwrap();
        let (decoded_data, _) = super::BatchCompressInstructionData::zero_copy_at(&vec).unwrap();
        assert_eq!(decoded_data.pubkeys.len(), 2);
        assert_eq!(decoded_data.amounts.len(), 2);
        assert_eq!(*decoded_data.lamports.unwrap(), U64::from(3));
        for (i, pubkey) in decoded_data.pubkeys.iter().enumerate() {
            assert_eq!(data.pubkeys[i], pubkey.into(),);
        }
        for (i, amount) in decoded_data.amounts.iter().enumerate() {
            assert_eq!(amount.get(), data.amounts[i]);
        }
    }

    #[test]
    fn test_batch_compress_instruction_data_none() {
        let data = super::BatchCompressInstructionDataBorsh {
            pubkeys: vec![Pubkey::new_unique(), Pubkey::new_unique()],
            amounts: vec![1, 2],
            lamports: None,
        };
        let mut vec = Vec::new();
        data.serialize(&mut vec).unwrap();
        let (decoded_data, _) = super::BatchCompressInstructionData::zero_copy_at(&vec).unwrap();
        assert_eq!(decoded_data.pubkeys.len(), 2);
        assert_eq!(decoded_data.amounts.len(), 2);
        assert!(decoded_data.lamports.is_none());
        for (i, pubkey) in decoded_data.pubkeys.iter().enumerate() {
            assert_eq!(data.pubkeys[i], (*pubkey).into(),);
        }
        for (i, amount) in decoded_data.amounts.iter().enumerate() {
            assert_eq!(amount.get(), data.amounts[i]);
        }
    }
}
