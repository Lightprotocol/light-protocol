use anchor_lang::{prelude::*, AnchorDeserialize, AnchorSerialize};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError, slice::ZeroCopySliceBorsh};
use zerocopy::{little_endian::U64, Ref};

#[derive(Debug, Default, Clone, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct BatchCompressInstructionDataBorsh {
    pub pubkeys: Vec<Pubkey>,
    pub amounts: Option<Vec<u64>>,
    pub lamports: Option<u64>,
    pub amount: Option<u64>,
    pub index: u8,
}

pub struct BatchCompressInstructionData<'a> {
    pub pubkeys: ZeroCopySliceBorsh<'a, light_compressed_account::pubkey::Pubkey>,
    pub amounts: Option<ZeroCopySliceBorsh<'a, U64>>,
    pub lamports: Option<Ref<&'a [u8], U64>>,
    pub amount: Option<Ref<&'a [u8], U64>>,
    pub index: u8,
}

impl<'a> Deserialize<'a> for BatchCompressInstructionData<'a> {
    type Output = Self;

    fn zero_copy_at(bytes: &'a [u8]) -> std::result::Result<(Self, &'a [u8]), ZeroCopyError> {
        let (pubkeys, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
        let (amounts, bytes) = Option::<ZeroCopySliceBorsh<U64>>::zero_copy_at(bytes)?;
        let (lamports, bytes) = Option::<U64>::zero_copy_at(bytes)?;
        let (amount, bytes) = Option::<U64>::zero_copy_at(bytes)?;
        let (index, bytes) = u8::zero_copy_at(bytes)?;
        Ok((
            Self {
                pubkeys,
                amounts,
                lamports,
                amount,
                index,
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
            amounts: Some(vec![1, 2]),
            lamports: Some(3),
            amount: Some(1),
            index: 1,
        };
        let mut vec = Vec::new();
        data.serialize(&mut vec).unwrap();
        let (decoded_data, _) = super::BatchCompressInstructionData::zero_copy_at(&vec).unwrap();
        assert_eq!(decoded_data.pubkeys.len(), 2);
        assert_eq!(decoded_data.amounts.as_ref().unwrap().len(), 2);
        assert_eq!(*decoded_data.lamports.unwrap(), U64::from(3));
        for (i, pubkey) in decoded_data.pubkeys.iter().enumerate() {
            assert_eq!(data.pubkeys[i], pubkey.into(),);
        }
        for (i, amount) in decoded_data.amounts.as_ref().unwrap().iter().enumerate() {
            assert_eq!(amount.get(), data.amounts.as_ref().unwrap()[i]);
        }
        assert_eq!(decoded_data.index, 1);
        assert_eq!(*decoded_data.amount.unwrap(), data.amount.unwrap());
    }

    #[test]
    fn test_batch_compress_instruction_data_none() {
        let data = super::BatchCompressInstructionDataBorsh {
            pubkeys: vec![Pubkey::new_unique(), Pubkey::new_unique()],
            amounts: Some(vec![1, 2]),
            amount: None,
            lamports: None,
            index: 0,
        };
        let mut vec = Vec::new();
        data.serialize(&mut vec).unwrap();
        let (decoded_data, _) = super::BatchCompressInstructionData::zero_copy_at(&vec).unwrap();
        assert_eq!(decoded_data.pubkeys.len(), 2);
        assert_eq!(decoded_data.amounts.as_ref().unwrap().len(), 2);
        assert!(decoded_data.lamports.is_none());
        for (i, pubkey) in decoded_data.pubkeys.iter().enumerate() {
            assert_eq!(data.pubkeys[i], (*pubkey).into(),);
        }
        for (i, amount) in decoded_data.amounts.as_ref().unwrap().iter().enumerate() {
            assert_eq!(amount.get(), data.amounts.as_ref().unwrap()[i]);
        }
        assert_eq!(decoded_data.index, 0);
        assert_eq!(decoded_data.amount, None);
    }
}
