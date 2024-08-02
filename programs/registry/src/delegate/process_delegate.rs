use anchor_lang::prelude::*;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::compressed_account::{CompressedAccount, PackedCompressedAccountWithMerkleContext},
    OutputCompressedAccountWithPackedContext,
};

use crate::{errors::RegistryError, protocol_config::state::ProtocolConfig, ForesterAccount};

use super::{
    delegate_instruction::DelegatetOrUndelegateInstruction,
    process_cpi::cpi_light_system_program,
    process_deposit::{
        create_compressed_delegate_account, create_delegate_compressed_account,
        DelegateAccountWithPackedContext,
    },
};

// TODO: double check that we provide the possibility to pass a different output tree in all instructions
pub fn process_delegate_or_undelegate<'a, 'b, 'c, 'info: 'b + 'c, const IS_DELEGATE: bool>(
    ctx: Context<'a, 'b, 'c, 'info, DelegatetOrUndelegateInstruction<'info>>,
    proof: CompressedProof,
    delegate_account: DelegateAccountWithPackedContext,
    delegate_amount: u64,
) -> Result<()> {
    let slot = Clock::get()?.slot;
    let (input_delegate_pda, output_delegate_pda) = delegate_or_undelegate::<IS_DELEGATE>(
        &ctx.accounts.authority.key(),
        &ctx.accounts.protocol_config.config,
        delegate_account,
        &ctx.accounts.forester_pda.key(),
        &mut ctx.accounts.forester_pda,
        delegate_amount,
        slot,
    )?;

    cpi_light_system_program(
        &ctx,
        Some(proof),
        None,
        Some(input_delegate_pda),
        output_delegate_pda,
        ctx.remaining_accounts.to_vec(),
    )
}

/// Delegate account has to be synced (sync_delegate instruction) to the last
/// claimed epoch of the forester to delegate or undelegate.
/// (Un)Delegation of newly (un)delegated funds should come into effect in the next epoch (registration phase).
pub fn delegate_or_undelegate<const IS_DELEGATE: bool>(
    authority: &Pubkey,
    protocol_config: &ProtocolConfig,
    mut delegate_account: DelegateAccountWithPackedContext,
    forester_pda_pubkey: &Pubkey,
    forester_pda: &mut ForesterAccount,
    delegate_amount: u64,
    current_slot: u64,
) -> Result<(
    PackedCompressedAccountWithMerkleContext,
    OutputCompressedAccountWithPackedContext,
)> {
    forester_pda.sync(current_slot, protocol_config)?;

    if *authority != delegate_account.delegate_account.owner {
        return err!(RegistryError::InvalidAuthority);
    }

    /**
     * Scenario 1: Delegate to inactive forester which never does anything after delegation
     * - it takes one epoch for stake to become active
     * - after one epoch it should be possible to undelegate
     *
     * Scenario 2: Delegate to somehwat active forester
     * - it takes one epoch for stake to become active
     * - after one epoch it should be possible to undelegate
     * - forester was active in the last epoch but doesn't claim
     * - last_registered_epoch = epoch or epoch - n
     * - last claimed epoch = epoch - n
     * last claimed epoch is the epoch to which the delegate account needs to be synced
     */
    // The account needs to be synced to a certain degree so that the sync delegate function works.
    // Edge cases:
    // - Forester never registered and claimed after delegation
    //   - (add option for last_claimed_epoch sync is not checked if last claimed is not set) this doesn't work if the forester has registered and claims later
    // - never claims after delegation
    // check that delegate account is synced to last claimed (completed) epoch
    if forester_pda.last_claimed_epoch != delegate_account.delegate_account.last_sync_epoch
        && delegate_account
            .delegate_account
            .delegate_forester_delegate_account
            .is_some()
    {
        msg!(
            "Not synced to last forester claimed epoch {}, last synced epoch {} ",
            forester_pda.last_claimed_epoch,
            delegate_account.delegate_account.last_sync_epoch
        );
        return err!(RegistryError::DelegateAccountNotSynced);
    }
    if let Some(forester_pubkey) = delegate_account
        .delegate_account
        .delegate_forester_delegate_account
    {
        if forester_pubkey != *forester_pda_pubkey {
            msg!("The delegate account is delegated to a different forester. The provided forester pda is not the same as the one in the delegate account.");
            return err!(RegistryError::AlreadyDelegated);
        }
    }
    let epoch =         // In case of delegating to an inactive forester, the delegate account needs to be synced so that.
    if forester_pda.last_registered_epoch <= delegate_account.delegate_account.last_sync_epoch
        || forester_pda.last_claimed_epoch <= delegate_account.delegate_account.last_sync_epoch
    {
        forester_pda.current_epoch
    } else {
        forester_pda.last_registered_epoch
    };
    msg!("epoch: {}", epoch);
    // modify forester pda
    if IS_DELEGATE {
        forester_pda.pending_undelegated_stake_weight = forester_pda
            .pending_undelegated_stake_weight
            .checked_add(delegate_amount)
            .ok_or(RegistryError::ComputeEscrowAmountFailed)?;
    } else {
        msg!(
            "forester pda active stake weight: {}",
            forester_pda.active_stake_weight
        );
        msg!("undelegate amount {}", delegate_amount);
        forester_pda.active_stake_weight = forester_pda
            .active_stake_weight
            .checked_sub(delegate_amount)
            .ok_or(RegistryError::ComputeEscrowAmountFailed)?;
    }

    // modify delegate account
    let delegate_account_mod = {
        let mut delegate_account = delegate_account.delegate_account;
        delegate_account.sync_pending_stake_weight(epoch);
        if IS_DELEGATE {
            // add delegated stake weight to pending_delegated_stake_weight
            // remove delegated stake weight from stake_weight
            delegate_account.pending_delegated_stake_weight = delegate_account
                .pending_delegated_stake_weight
                .checked_add(delegate_amount)
                .ok_or(RegistryError::ComputeEscrowAmountFailed)?;
            delegate_account.stake_weight = delegate_account
                .stake_weight
                .checked_sub(delegate_amount)
                .ok_or(RegistryError::ComputeEscrowAmountFailed)?;
            if delegate_account
                .delegate_forester_delegate_account
                .is_none()
            {
                delegate_account.delegate_forester_delegate_account = Some(*forester_pda_pubkey);
                delegate_account.last_sync_epoch = forester_pda.last_claimed_epoch;
            }
            delegate_account.pending_epoch = epoch;
        } else {
            msg!(
                "delegate account delegated stake weight: {}",
                delegate_account.delegated_stake_weight
            );
            msg!("delegate amount {}", delegate_amount);
            // remove delegated stake weight from delegated_stake_weight
            // add delegated stake weight to pending_undelegated_stake_weight
            delegate_account.delegated_stake_weight = delegate_account
                .delegated_stake_weight
                .checked_sub(delegate_amount)
                .ok_or(RegistryError::ComputeEscrowAmountFailed)?;
            delegate_account.pending_undelegated_stake_weight = delegate_account
                .pending_undelegated_stake_weight
                .checked_add(delegate_amount)
                .ok_or(RegistryError::ComputeEscrowAmountFailed)?;
            delegate_account.pending_epoch = epoch;
        }
        delegate_account
    };
    let input_delegate_compressed_account = create_compressed_delegate_account(
        delegate_account.delegate_account,
        delegate_account.merkle_context,
        delegate_account.root_index,
    )?;
    let output_account: CompressedAccount =
        create_delegate_compressed_account::<false>(&delegate_account_mod)?;
    let output_delegate_compressed_account = OutputCompressedAccountWithPackedContext {
        compressed_account: output_account,
        merkle_tree_index: delegate_account.output_merkle_tree_index,
    };

    Ok((
        input_delegate_compressed_account,
        output_delegate_compressed_account,
    ))
}

// /// Creates an updated delegate account.
// /// Delegate(IS_DELEGATE):
// /// - increase delegated_stake_weight
// /// - decrease stake_weight
// /// Undelegate(Not(IS_DELEGATE)):
// /// - decrease delegated_stake_weight
// /// - increase pending_undelegated_stake_weight
// fn update_delegate_compressed_account<const IS_DELEGATE: bool>(
//     input_delegate_account: DelegateAccountWithPackedContext,
//     delegate_amount: u64,
//     merkle_tree_index: u8,
//     epoch: u64,
//     forester_pda_pubkey: &Pubkey,
// ) -> Result<OutputCompressedAccountWithPackedContext> {
//     let output_account: CompressedAccount =
//         create_delegate_compressed_account::<false>(&input_delegate_account.delegate_account)?;
//     let output_account_with_merkle_context = OutputCompressedAccountWithPackedContext {
//         compressed_account: output_account,
//         merkle_tree_index,
//     };
//     Ok(output_account_with_merkle_context)
// }

#[cfg(test)]
mod tests {
    use crate::delegate::delegate_account::DelegateAccount;

    use super::*;
    use anchor_lang::solana_program::pubkey::Pubkey;
    // use light_hasher::{DataHasher, Poseidon};
    use light_system_program::sdk::compressed_account::PackedMerkleContext;

    fn get_test_delegate_account_with_context(
        protocol_config: &ProtocolConfig,
        current_slot: u64,
    ) -> DelegateAccountWithPackedContext {
        let current_epoch = protocol_config.get_current_registration_epoch(current_slot);

        DelegateAccountWithPackedContext {
            root_index: 4,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                nullifier_queue_pubkey_index: 2,
                leaf_index: 3,
                queue_index: None,
            },
            delegate_account: DelegateAccount {
                owner: Pubkey::new_unique(),
                delegate_forester_delegate_account: Some(Pubkey::new_unique()),
                delegated_stake_weight: 100,
                stake_weight: 200,
                pending_delegated_stake_weight: 0,
                pending_undelegated_stake_weight: 50,
                pending_epoch: 1,
                last_sync_epoch: current_epoch - 1,
                pending_token_amount: 25,
                escrow_token_account_hash: [1u8; 32],
                pending_synced_stake_weight: 0,
            },
            output_merkle_tree_index: 6,
        }
    }

    fn get_test_forester_account(
        protocol_config: &ProtocolConfig,
        current_slot: u64,
    ) -> ForesterAccount {
        let current_epoch = protocol_config.get_current_registration_epoch(current_slot);
        ForesterAccount {
            active_stake_weight: 200,
            pending_undelegated_stake_weight: 50,
            current_epoch: current_epoch - 1,
            last_claimed_epoch: current_epoch - 1,
            last_registered_epoch: current_epoch - 1,
            ..Default::default()
        }
    }

    /// Failing tests:
    /// 1. Invalid authority
    /// 2. Delegate account not synced
    /// 3. Invalid forester
    /// 4. Already delegated
    /// Functional tests:
    /// 1. Outputs are created as expected (rnd test for this)
    #[test]
    fn test_functional_delegate() {
        let (
            protocol_config,
            current_slot,
            mut forester_pda,
            mut expected_forester_pda,
            delegate_account,
            authority,
            forester_pda_pubkey,
        ) = test_setup();
        let delegate_amount = 50;

        let result = delegate_or_undelegate::<true>(
            &authority,
            &protocol_config,
            delegate_account,
            &forester_pda_pubkey,
            &mut forester_pda,
            delegate_amount,
            current_slot,
        );

        // This test is currently failing because the delegate the syncing I added changes the delegate account in a different way than before.
        let (input_delegate_pda, output_delegate_pda) = result.unwrap();
        assert_eq!(input_delegate_pda.compressed_account.owner, crate::ID);
        assert_eq!(output_delegate_pda.compressed_account.owner, crate::ID);
        // TODO: test sync pending stake weight
        // Delegate should:
        // - sync pending stake weight
        // - output pending
        let mut expected_delegate_account = DelegateAccount {
            pending_delegated_stake_weight: delegate_amount,
            pending_epoch: forester_pda.last_registered_epoch,
            stake_weight: delegate_account.delegate_account.stake_weight - delegate_amount,
            ..delegate_account.delegate_account
        };
        println!("epoch {}", expected_forester_pda.current_epoch);
        expected_delegate_account.sync_pending_stake_weight(expected_forester_pda.current_epoch);

        let deserialized_delegate_account = DelegateAccount::deserialize(
            &mut &output_delegate_pda
                .compressed_account
                .data
                .as_ref()
                .unwrap()
                .data[..],
        )
        .unwrap();
        assert_eq!(deserialized_delegate_account, expected_delegate_account);
        expected_forester_pda
            .sync(current_slot, &protocol_config)
            .unwrap();
        assert_eq!(forester_pda, expected_forester_pda);
    }

    fn test_setup() -> (
        ProtocolConfig,
        u64,
        ForesterAccount,
        ForesterAccount,
        DelegateAccountWithPackedContext,
        Pubkey,
        Pubkey,
    ) {
        let protocol_config = ProtocolConfig {
            ..Default::default()
        };
        // slot in active phase of epoch 2
        let current_slot = protocol_config.genesis_slot
            + protocol_config.registration_phase_length
            + protocol_config.active_phase_length * 2
            + 1;
        let forester_pda = get_test_forester_account(&protocol_config, current_slot);
        // setting current epoch to -1 to test that it is synced
        assert_eq!(forester_pda.current_epoch, 1);
        let mut expected_forester_pda = forester_pda.clone();
        expected_forester_pda.current_epoch = 2;
        expected_forester_pda.active_stake_weight +=
            expected_forester_pda.pending_undelegated_stake_weight;
        let delegate_account =
            get_test_delegate_account_with_context(&protocol_config, current_slot);
        let authority = delegate_account.delegate_account.owner;
        let forester_pda_pubkey = delegate_account
            .delegate_account
            .delegate_forester_delegate_account
            .unwrap();
        (
            protocol_config,
            current_slot,
            forester_pda,
            expected_forester_pda,
            delegate_account,
            authority,
            forester_pda_pubkey,
        )
    }

    #[test]
    fn test_functional_undelegate() {
        let (
            protocol_config,
            current_slot,
            mut forester_pda,
            mut expected_forester_pda,
            delegate_account,
            authority,
            forester_pda_pubkey,
        ) = test_setup();
        let delegate_amount = 50;
        // let current_slot = 10;
        println!("pre forester_pda {:?}", forester_pda);
        let result = delegate_or_undelegate::<false>(
            &authority,
            &protocol_config,
            delegate_account,
            &forester_pda_pubkey,
            &mut forester_pda,
            delegate_amount,
            current_slot,
        )
        .unwrap();

        let (input_delegate_pda, output_delegate_pda) = result;
        assert_eq!(input_delegate_pda.compressed_account.owner, crate::ID);
        assert_eq!(output_delegate_pda.compressed_account.owner, crate::ID);

        let expected_delegate_account = DelegateAccount {
            delegated_stake_weight: delegate_account.delegate_account.delegated_stake_weight
                - delegate_amount,
            pending_undelegated_stake_weight: delegate_account
                .delegate_account
                .pending_undelegated_stake_weight
                + delegate_amount,
            pending_epoch: forester_pda.last_registered_epoch,
            ..delegate_account.delegate_account
        };

        let deserialized_delegate_account = DelegateAccount::deserialize(
            &mut &output_delegate_pda
                .compressed_account
                .data
                .as_ref()
                .unwrap()
                .data[..],
        )
        .unwrap();
        assert_eq!(deserialized_delegate_account, expected_delegate_account);
        expected_forester_pda
            .sync(current_slot, &protocol_config)
            .unwrap();
        expected_forester_pda.pending_undelegated_stake_weight -= delegate_amount;
        expected_forester_pda.active_stake_weight -= delegate_amount;

        assert_eq!(forester_pda, expected_forester_pda);
    }

    #[test]
    fn test_delegate_or_undelegate_undelegate_fail() {
        let (
            protocol_config,
            current_slot,
            mut forester_pda,
            mut expected_forester_pda,
            delegate_account,
            authority,
            forester_pda_pubkey,
        ) = test_setup();
        let authority = Pubkey::new_unique();
        let forester_pda_pubkey = Pubkey::new_unique();
        let delegate_amount = u64::MAX;
        let current_slot = 10;

        let result = delegate_or_undelegate::<false>(
            &authority,
            &protocol_config,
            delegate_account,
            &forester_pda_pubkey,
            &mut forester_pda,
            delegate_amount,
            current_slot,
        );

        assert!(matches!(result, Err(error) if error == RegistryError::InvalidAuthority.into()));
    }

    #[test]
    fn test_delegate_or_undelegate_delegate_fail() {
        let (
            protocol_config,
            current_slot,
            mut forester_pda,
            mut expected_forester_pda,
            delegate_account,
            authority,
            forester_pda_pubkey,
        ) = test_setup();
        let forester_pda_pubkey = Pubkey::new_unique();
        // let authority = delegate_account.delegate_account.owner;
        let delegate_amount = u64::MAX;
        let current_slot = 10;

        let result = delegate_or_undelegate::<true>(
            &authority,
            &protocol_config,
            delegate_account,
            &forester_pda_pubkey,
            &mut forester_pda,
            delegate_amount,
            current_slot,
        );
        println!("{:?}", result);
        assert!(matches!(result, Err(error) if error == RegistryError::AlreadyDelegated.into()));
    }
}
