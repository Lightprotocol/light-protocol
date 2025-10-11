use anchor_lang::{prelude::*, AnchorDeserialize, AnchorSerialize};
use light_zero_copy::{errors::ZeroCopyError, slice::ZeroCopySliceBorsh, traits::ZeroCopyAt};
use zerocopy::{little_endian::U64, Ref};

#[derive(Debug, Default, Clone, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct BatchCompressInstructionDataBorsh {
    pub pubkeys: Vec<Pubkey>,
    // Some if one amount per pubkey.
    pub amounts: Option<Vec<u64>>,
    pub lamports: Option<u64>,
    // Some if one amount across all pubkeys.
    pub amount: Option<u64>,
    pub index: u8,
    pub bump: u8,
}

pub struct BatchCompressInstructionData<'a> {
    pub pubkeys: ZeroCopySliceBorsh<'a, light_compressed_account::pubkey::Pubkey>,
    pub amounts: Option<ZeroCopySliceBorsh<'a, U64>>,
    pub lamports: Option<Ref<&'a [u8], U64>>,
    pub amount: Option<Ref<&'a [u8], U64>>,
    pub index: u8,
    pub bump: u8,
}

impl<'a> ZeroCopyAt<'a> for BatchCompressInstructionData<'a> {
    type ZeroCopyAt = Self;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> std::result::Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (pubkeys, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
        let (amounts, bytes) = Option::<ZeroCopySliceBorsh<U64>>::zero_copy_at(bytes)?;
        let (lamports, bytes) = Option::<U64>::zero_copy_at(bytes)?;
        let (amount, bytes) = Option::<U64>::zero_copy_at(bytes)?;
        let (index, bytes) = u8::zero_copy_at(bytes)?;
        let (bump, bytes) = u8::zero_copy_at(bytes)?;
        Ok((
            Self {
                pubkeys,
                amounts,
                lamports,
                amount,
                index,
                bump,
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
            bump: 2,
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
        assert_eq!(decoded_data.bump, data.bump);
    }

    #[test]
    fn test_batch_compress_instruction_data_none() {
        let data = super::BatchCompressInstructionDataBorsh {
            pubkeys: vec![Pubkey::new_unique(), Pubkey::new_unique()],
            amounts: Some(vec![1, 2]),
            amount: None,
            lamports: None,
            index: 0,
            bump: 0,
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
        assert_eq!(decoded_data.bump, data.bump);
    }

    #[test]
    fn test_batch_compress_instruction_data_randomized() {
        use rand::Rng;

        for _ in 0..100000 {
            let mut rng = rand::thread_rng();

            let pubkeys_count = rng.gen_range(1..10);
            let pubkeys: Vec<Pubkey> = (0..pubkeys_count).map(|_| Pubkey::new_unique()).collect();

            let amounts = if rng.gen_bool(0.5) {
                Some((0..pubkeys_count).map(|_| rng.gen_range(1..1000)).collect())
            } else {
                None
            };

            let lamports = if rng.gen_bool(0.5) {
                Some(rng.gen_range(1..1000))
            } else {
                None
            };

            let amount = if rng.gen_bool(0.5) {
                Some(rng.gen_range(1..1000))
            } else {
                None
            };

            let index = rng.gen_range(0..=u8::MAX);
            let bump = rng.gen_range(0..=u8::MAX);

            let data = super::BatchCompressInstructionDataBorsh {
                pubkeys,
                amounts,
                lamports,
                amount,
                index,
                bump,
            };

            let mut vec = Vec::new();
            data.serialize(&mut vec).unwrap();
            let (decoded_data, _) =
                super::BatchCompressInstructionData::zero_copy_at(&vec).unwrap();

            assert_eq!(decoded_data.pubkeys.len(), data.pubkeys.len());
            if let Some(amounts) = &data.amounts {
                assert_eq!(decoded_data.amounts.as_ref().unwrap().len(), amounts.len());
                for (i, amount) in decoded_data.amounts.as_ref().unwrap().iter().enumerate() {
                    assert_eq!(amount.get(), amounts[i]);
                }
            } else {
                assert!(decoded_data.amounts.is_none());
            }

            if let Some(lamports) = data.lamports {
                assert_eq!(*decoded_data.lamports.unwrap(), U64::from(lamports));
            } else {
                assert!(decoded_data.lamports.is_none());
            }

            if let Some(amount) = data.amount {
                assert_eq!(*decoded_data.amount.unwrap(), U64::from(amount));
            } else {
                assert!(decoded_data.amount.is_none());
            }

            for (i, pubkey) in decoded_data.pubkeys.iter().enumerate() {
                assert_eq!(data.pubkeys[i], (*pubkey).into());
            }

            assert_eq!(decoded_data.index, data.index);
            assert_eq!(decoded_data.bump, data.bump);
        }
    }
}
