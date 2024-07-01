use anchor_lang::prelude::*;

#[account(zero_copy)]
#[derive(AnchorDeserialize, Debug, PartialEq, Default)]
pub struct RolloverMetadata {
    /// Unique index.
    pub index: u64,
    /// This fee is used for rent for the next account.
    /// It accumulates in the account so that once the corresponding Merkle tree account is full it can be rolled over
    pub rollover_fee: u64,
    /// The threshold in percentage points when the account should be rolled over (95 corresponds to 95% filled).
    pub rollover_threshold: u64,
    /// Tip for maintaining the account.
    pub network_fee: u64,
    /// The slot when the account was rolled over, a rolled over account should not be written to.
    pub rolledover_slot: u64,
    /// If current slot is greater than rolledover_slot + close_threshold and
    /// the account is empty it can be closed. No 'close' functionality has been
    /// implemented yet.
    pub close_threshold: u64,
}

impl RolloverMetadata {
    pub fn new(
        index: u64,
        rollover_fee: u64,
        rollover_threshold: Option<u64>,
        network_fee: u64,
        close_threshold: Option<u64>,
    ) -> Self {
        Self {
            index,
            rollover_fee,
            rollover_threshold: rollover_threshold.unwrap_or(u64::MAX),
            network_fee,
            rolledover_slot: u64::MAX,
            close_threshold: close_threshold.unwrap_or(u64::MAX),
        }
    }

    pub fn rollover(&mut self) -> Result<()> {
        if self.rollover_threshold == u64::MAX {
            return err!(crate::errors::AccountCompressionErrorCode::RolloverNotConfigured);
        }
        if self.rolledover_slot != u64::MAX {
            return err!(crate::errors::AccountCompressionErrorCode::MerkleTreeAlreadyRolledOver);
        }

        #[cfg(target_os = "solana")]
        {
            self.rolledover_slot = Clock::get()?.slot;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollover_metadata() {
        let mut metadata = RolloverMetadata::new(0, 0, Some(95), 0, Some(100));
        assert_eq!(metadata.rollover_threshold, 95);
        assert_eq!(metadata.close_threshold, 100);
        assert_eq!(metadata.rolledover_slot, u64::MAX);

        metadata.rollover().unwrap();

        let mut metadata = RolloverMetadata::new(0, 0, None, 0, None);
        assert_eq!(metadata.rollover_threshold, u64::MAX);
        assert_eq!(metadata.close_threshold, u64::MAX);

        assert_eq!(
            metadata.rollover(),
            Err(crate::errors::AccountCompressionErrorCode::RolloverNotConfigured.into())
        );
        let mut metadata = RolloverMetadata::new(0, 0, Some(95), 0, None);
        assert_eq!(metadata.close_threshold, u64::MAX);

        metadata.rollover().unwrap();
        let mut metadata = RolloverMetadata::new(0, 0, Some(95), 0, None);
        metadata.rolledover_slot = 0;
        assert_eq!(metadata.close_threshold, u64::MAX);

        assert_eq!(
            metadata.rollover(),
            Err(crate::errors::AccountCompressionErrorCode::MerkleTreeAlreadyRolledOver.into())
        );
    }
}
