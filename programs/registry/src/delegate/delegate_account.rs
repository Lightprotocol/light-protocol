use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_hasher::{errors::HasherError, DataHasher, Hasher};
use light_utils::hash_to_bn254_field_size_be;

/// Instruction data input verion of DelegateAccount The following fields are
/// missing since these are computed onchain:
/// 1. owner
/// 2. escrow_token_account_hash
/// -> we save 64 bytes in instructiond data
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct InputDelegateAccount {
    pub delegate_forester_delegate_account: Option<Pubkey>,
    /// Stake weight that is delegated to a forester.
    /// Newly delegated stake is not active until the next epoch.
    pub delegated_stake_weight: u64,
    /// When delegating stake is pending until the next epoch
    pub pending_delegated_stake_weight: u64,
    /// undelgated stake is stake that is not yet delegated to a forester
    pub stake_weight: u64,
    pub pending_synced_stake_weight: u64,
    /// When undelegating stake is pending until the next epoch
    pub pending_undelegated_stake_weight: u64,
    pub pending_epoch: u64,
    pub last_sync_epoch: u64,
    /// Pending token amount are rewards that are not yet claimed to the stake
    /// compressed token account.
    pub pending_token_amount: u64,
}

impl From<DelegateAccount> for InputDelegateAccount {
    fn from(delegate_account: DelegateAccount) -> Self {
        InputDelegateAccount {
            delegate_forester_delegate_account: delegate_account.delegate_forester_delegate_account,
            delegated_stake_weight: delegate_account.delegated_stake_weight,
            stake_weight: delegate_account.stake_weight,
            pending_undelegated_stake_weight: delegate_account.pending_undelegated_stake_weight,
            pending_epoch: delegate_account.pending_epoch,
            last_sync_epoch: delegate_account.last_sync_epoch,
            pending_token_amount: delegate_account.pending_token_amount,
            pending_synced_stake_weight: delegate_account.pending_synced_stake_weight,
            pending_delegated_stake_weight: delegate_account.pending_delegated_stake_weight,
        }
    }
}

#[aligned_sized]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct DelegateAccount {
    pub owner: Pubkey,
    pub delegate_forester_delegate_account: Option<Pubkey>,
    /// Stake weight that is delegated to a forester.
    /// Newly delegated stake is not active until the next epoch.
    pub delegated_stake_weight: u64,
    /// newly delegated stakeweight becomes active after the next epoch
    pub pending_delegated_stake_weight: u64,
    /// undelgated stake is stake that is not yet delegated to a forester
    pub stake_weight: u64,
    /// Buffer variable to account for the lag of one epoch for rewards to reach
    /// to registration account
    pub pending_synced_stake_weight: u64,
    /// When undelegating stake is pending until the next epoch
    pub pending_undelegated_stake_weight: u64,
    pub pending_epoch: u64,
    pub last_sync_epoch: u64,
    /// Pending token amount are rewards that are not yet claimed to the stake
    /// compressed token account.
    pub pending_token_amount: u64,
    pub escrow_token_account_hash: [u8; 32],
}

pub trait CompressedAccountTrait {
    fn get_owner(&self) -> Pubkey;
}
impl CompressedAccountTrait for DelegateAccount {
    fn get_owner(&self) -> Pubkey {
        self.owner
    }
}

impl DataHasher for DelegateAccount {
    fn hash<H: Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
        let hashed_owner = hash_to_bn254_field_size_be(self.owner.as_ref()).unwrap().0;
        let hashed_delegate_forester_delegate_account =
            if let Some(delegate_forester_delegate_account) =
                self.delegate_forester_delegate_account
            {
                hash_to_bn254_field_size_be(delegate_forester_delegate_account.as_ref())
                    .unwrap()
                    .0
            } else {
                [0u8; 32]
            };
        H::hashv(&[
            hashed_owner.as_slice(),
            hashed_delegate_forester_delegate_account.as_slice(),
            &self.delegated_stake_weight.to_le_bytes(),
            &self.pending_delegated_stake_weight.to_le_bytes(),
            &self.stake_weight.to_le_bytes(),
            &self.pending_synced_stake_weight.to_le_bytes(),
            &self.pending_undelegated_stake_weight.to_le_bytes(),
            &self.pending_epoch.to_le_bytes(),
            &self.last_sync_epoch.to_le_bytes(),
            &self.pending_token_amount.to_le_bytes(),
            &self.escrow_token_account_hash,
        ])
    }
}

impl DelegateAccount {
    // TODO: add unit test
    pub fn sync_pending_stake_weight(&mut self, current_epoch: u64) {
        println!("current_epoch: {}", current_epoch);
        println!("pending_epoch: {}", self.pending_epoch);
        #[cfg(target_os = "solana")]
        {
            msg!("current_epoch: {}", current_epoch);
            msg!("pending_epoch: {}", self.pending_epoch);
        }
        if current_epoch > self.pending_epoch {
            self.stake_weight = self
                .stake_weight
                .checked_add(self.pending_undelegated_stake_weight)
                .unwrap();
            self.pending_undelegated_stake_weight = 0;
            self.delegated_stake_weight = self
                .delegated_stake_weight
                .checked_add(self.pending_delegated_stake_weight)
                .unwrap();
            self.pending_delegated_stake_weight = 0;
            if self.delegated_stake_weight == 0 {
                self.delegate_forester_delegate_account = None;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use light_hasher::Poseidon;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn failing_test_hashing_delegate_account() {
        let mut vec_previous_hashes = Vec::new();
        let delegate_account = DelegateAccount {
            owner: Pubkey::new_unique(),
            delegate_forester_delegate_account: None,
            delegated_stake_weight: 1000,
            pending_delegated_stake_weight: 500,
            stake_weight: 1500,
            pending_synced_stake_weight: 200,
            pending_undelegated_stake_weight: 100,
            pending_epoch: 1,
            last_sync_epoch: 2,
            pending_token_amount: 50,
            escrow_token_account_hash: [0u8; 32],
        };
        let hash = delegate_account.hash::<Poseidon>().unwrap();
        vec_previous_hashes.push(hash);

        // different owner
        let mut different_owner_account = delegate_account;
        different_owner_account.owner = Pubkey::new_unique();
        let hash2 = different_owner_account.hash::<Poseidon>().unwrap();
        assert_to_previous_hashes(hash2, &mut vec_previous_hashes);

        // different delegate_forester_delegate_account
        let mut different_delegate_account = delegate_account;
        different_delegate_account.delegate_forester_delegate_account = Some(Pubkey::new_unique());
        let hash3 = different_delegate_account.hash::<Poseidon>().unwrap();
        assert_to_previous_hashes(hash3, &mut vec_previous_hashes);

        // different other delegate_forester_delegate_account (since initial value is None)
        let mut different_delegate_account = delegate_account;
        different_delegate_account.delegate_forester_delegate_account = Some(Pubkey::new_unique());
        let hash3 = different_delegate_account.hash::<Poseidon>().unwrap();
        assert_to_previous_hashes(hash3, &mut vec_previous_hashes);

        // different delegated_stake_weight
        let mut different_stake_weight_account = delegate_account;
        different_stake_weight_account.delegated_stake_weight = 2000;
        let hash4 = different_stake_weight_account.hash::<Poseidon>().unwrap();
        assert_to_previous_hashes(hash4, &mut vec_previous_hashes);

        // different pending_delegated_stake_weight
        let mut different_pending_stake_weight_account = delegate_account;
        different_pending_stake_weight_account.pending_delegated_stake_weight = 1000;
        let hash5 = different_pending_stake_weight_account
            .hash::<Poseidon>()
            .unwrap();
        assert_to_previous_hashes(hash5, &mut vec_previous_hashes);

        // different stake_weight
        let mut different_stake_weight_account = delegate_account;
        different_stake_weight_account.stake_weight = 2500;
        let hash6 = different_stake_weight_account.hash::<Poseidon>().unwrap();
        assert_to_previous_hashes(hash6, &mut vec_previous_hashes);

        // different pending_synced_stake_weight
        let mut different_pending_synced_stake_weight_account = delegate_account;
        different_pending_synced_stake_weight_account.pending_synced_stake_weight = 300;
        let hash7 = different_pending_synced_stake_weight_account
            .hash::<Poseidon>()
            .unwrap();
        assert_to_previous_hashes(hash7, &mut vec_previous_hashes);

        // different pending_undelegated_stake_weight
        let mut different_pending_undelegated_stake_weight_account = delegate_account;
        different_pending_undelegated_stake_weight_account.pending_undelegated_stake_weight = 200;
        let hash8 = different_pending_undelegated_stake_weight_account
            .hash::<Poseidon>()
            .unwrap();
        assert_to_previous_hashes(hash8, &mut vec_previous_hashes);

        // different pending_epoch
        let mut different_pending_epoch_account = delegate_account;
        different_pending_epoch_account.pending_epoch = 3;
        let hash9 = different_pending_epoch_account.hash::<Poseidon>().unwrap();
        assert_to_previous_hashes(hash9, &mut vec_previous_hashes);

        // different last_sync_epoch
        let mut different_last_sync_epoch_account = delegate_account;
        different_last_sync_epoch_account.last_sync_epoch = 4;
        let hash10 = different_last_sync_epoch_account
            .hash::<Poseidon>()
            .unwrap();
        assert_to_previous_hashes(hash10, &mut vec_previous_hashes);

        // different pending_token_amount
        let mut different_pending_token_amount_account = delegate_account;
        different_pending_token_amount_account.pending_token_amount = 100;
        let hash11 = different_pending_token_amount_account
            .hash::<Poseidon>()
            .unwrap();
        assert_to_previous_hashes(hash11, &mut vec_previous_hashes);

        // different escrow_token_account_hash
        let mut different_escrow_token_account_hash = delegate_account;
        different_escrow_token_account_hash.escrow_token_account_hash = [1u8; 32];
        let hash12 = different_escrow_token_account_hash
            .hash::<Poseidon>()
            .unwrap();
        assert_to_previous_hashes(hash12, &mut vec_previous_hashes);
    }

    fn assert_to_previous_hashes(hash: [u8; 32], previous_hashes: &mut Vec<[u8; 32]>) {
        for previous_hash in previous_hashes.iter() {
            assert_ne!(hash, *previous_hash);
        }
        println!("len previous hashes: {}", previous_hashes.len());
        previous_hashes.push(hash);
    }
}
