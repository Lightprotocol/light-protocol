use anchor_lang::prelude::*;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::compressed_account::{CompressedAccount, PackedCompressedAccountWithMerkleContext},
    OutputCompressedAccountWithPackedContext,
};

use crate::{errors::RegistryError, protocol_config::state::ProtocolConfig, ForesterAccount};

use super::{
    delegate_instruction::DelegatetOrUndelegateInstruction,
    deposit::{
        create_compressed_delegate_account, create_delegate_compressed_account,
        DelegateAccountWithPackedContext,
    },
    process_cpi::cpi_light_system_program,
};

// TODO: double check that we provide the possibility to pass a different output tree in all instructions
pub fn process_delegate_or_undelegate<'a, 'b, 'c, 'info: 'b + 'c, const IS_DELEGATE: bool>(
    ctx: Context<'a, 'b, 'c, 'info, DelegatetOrUndelegateInstruction<'info>>,
    proof: CompressedProof,
    delegate_account: DelegateAccountWithPackedContext,
    delegate_amount: u64,
    no_sync: bool,
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
        no_sync,
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

pub fn delegate_or_undelegate<const IS_DELEGATE: bool>(
    authority: &Pubkey,
    protocol_config: &ProtocolConfig,
    delegate_account: DelegateAccountWithPackedContext,
    forester_pda_pubkey: &Pubkey,
    forester_pda: &mut ForesterAccount,
    delegate_amount: u64,
    current_slot: u64,
    no_sync: bool,
) -> Result<(
    PackedCompressedAccountWithMerkleContext,
    OutputCompressedAccountWithPackedContext,
)> {
    if !no_sync {
        forester_pda.sync(current_slot, protocol_config)?;
    }
    if *authority != delegate_account.delegate_account.owner {
        return err!(RegistryError::InvalidAuthority);
    }
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
            return err!(RegistryError::InvalidForester);
        }
    }
    let epoch = forester_pda.last_registered_epoch; // protocol_config.get_current_epoch(current_slot);

    // check that is not delegated to a different forester
    if delegate_account.delegate_account.delegated_stake_weight > 0
        && delegate_account
            .delegate_account
            .delegate_forester_delegate_account
            .is_some()
        && *forester_pda_pubkey
            != delegate_account
                .delegate_account
                .delegate_forester_delegate_account
                .unwrap()
    {
        return err!(RegistryError::AlreadyDelegated);
    }
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
            delegate_account.delegate_forester_delegate_account = Some(*forester_pda_pubkey);
            delegate_account.pending_epoch = epoch;
        } else {
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
            if delegate_account.delegated_stake_weight == 0 {
                delegate_account.delegate_forester_delegate_account = None;
            }
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
    // let output_delegate_compressed_account = update_delegate_compressed_account::<IS_DELEGATE>(
    //     *delegate_account,
    //     delegate_amount,
    //     delegate_account.output_merkle_tree_index,
    //     epoch,
    //     forester_pda_pubkey,
    // )?;

    Ok((
        input_delegate_compressed_account,
        output_delegate_compressed_account,
    ))
}

/// Creates an updated delegate account.
/// Delegate(IS_DELEGATE):
/// - increase delegated_stake_weight
/// - decrease stake_weight
/// Undelegate(Not(IS_DELEGATE)):
/// - decrease delegated_stake_weight
/// - increase pending_undelegated_stake_weight
fn update_delegate_compressed_account<const IS_DELEGATE: bool>(
    input_delegate_account: DelegateAccountWithPackedContext,
    delegate_amount: u64,
    merkle_tree_index: u8,
    epoch: u64,
    forester_pda_pubkey: &Pubkey,
) -> Result<OutputCompressedAccountWithPackedContext> {
    let output_account: CompressedAccount =
        create_delegate_compressed_account::<false>(&input_delegate_account.delegate_account)?;
    let output_account_with_merkle_context = OutputCompressedAccountWithPackedContext {
        compressed_account: output_account,
        merkle_tree_index,
    };
    Ok(output_account_with_merkle_context)
}

#[cfg(test)]
mod tests {
    use crate::delegate::state::DelegateAccount;

    use super::*;
    use anchor_lang::solana_program::pubkey::Pubkey;
    use light_hasher::{DataHasher, Poseidon};
    use light_system_program::sdk::compressed_account::PackedMerkleContext;

    fn get_test_delegate_account_with_context() -> DelegateAccountWithPackedContext {
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
                last_sync_epoch: 11,
                pending_token_amount: 25,
                escrow_token_account_hash: [1u8; 32],
                pending_synced_stake_weight: 0,
            },
            output_merkle_tree_index: 6,
        }
    }

    #[test]
    fn test_update_delegate_compressed_account_delegate_pass() {
        let input_delegate_account = get_test_delegate_account_with_context();
        let delegate_amount = 50;
        let merkle_tree_index = 1;
        let epoch = 10;
        let forester_pda_pubkey = Pubkey::new_unique();

        let result = update_delegate_compressed_account::<true>(
            input_delegate_account.clone(),
            delegate_amount,
            merkle_tree_index,
            epoch,
            &forester_pda_pubkey,
        );

        assert!(result.is_ok());

        let expected_delegate_account = DelegateAccount {
            delegated_stake_weight: input_delegate_account
                .delegate_account
                .delegated_stake_weight
                + delegate_amount,
            delegate_forester_delegate_account: Some(forester_pda_pubkey),
            stake_weight: input_delegate_account.delegate_account.stake_weight - delegate_amount,
            ..input_delegate_account.delegate_account
        };

        let output = result.unwrap();
        assert_eq!(output.merkle_tree_index, merkle_tree_index);
        let deserialized_delegate_account = DelegateAccount::deserialize(
            &mut &output.compressed_account.data.as_ref().unwrap().data[..],
        )
        .unwrap();
        assert_eq!(deserialized_delegate_account, expected_delegate_account);
        assert_eq!(
            output.compressed_account.data.unwrap().data_hash,
            expected_delegate_account.hash::<Poseidon>().unwrap()
        );
    }

    #[test]
    fn test_update_delegate_compressed_account_delegate_fail() {
        let input_delegate_account = get_test_delegate_account_with_context();
        let delegate_amount = u64::MAX;
        let merkle_tree_index = 1;
        let epoch = 10;
        let forester_pda_pubkey = Pubkey::new_unique();

        let result = update_delegate_compressed_account::<true>(
            input_delegate_account.clone(),
            delegate_amount,
            merkle_tree_index,
            epoch,
            &forester_pda_pubkey,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_update_delegate_compressed_account_undelegate_pass() {
        let input_delegate_account = get_test_delegate_account_with_context();
        let delegate_amount = 50;
        let merkle_tree_index = 1;
        let epoch = 10;
        let forester_pda_pubkey = Pubkey::new_unique();

        let result = update_delegate_compressed_account::<false>(
            input_delegate_account.clone(),
            delegate_amount,
            merkle_tree_index,
            epoch,
            &forester_pda_pubkey,
        );

        assert!(result.is_ok());

        let expected_delegate_account = DelegateAccount {
            delegated_stake_weight: input_delegate_account
                .delegate_account
                .delegated_stake_weight
                - delegate_amount,
            pending_undelegated_stake_weight: input_delegate_account
                .delegate_account
                .pending_undelegated_stake_weight
                + delegate_amount,
            pending_epoch: epoch,
            ..input_delegate_account.delegate_account
        };

        let output = result.unwrap();
        assert_eq!(output.merkle_tree_index, merkle_tree_index);
        let deserialized_delegate_account = DelegateAccount::deserialize(
            &mut &output.compressed_account.data.as_ref().unwrap().data[..],
        )
        .unwrap();
        assert_eq!(deserialized_delegate_account, expected_delegate_account);
        assert_eq!(
            output.compressed_account.data.unwrap().data_hash,
            expected_delegate_account.hash::<Poseidon>().unwrap()
        );
    }

    #[test]
    fn test_update_delegate_compressed_account_undelegate_fail() {
        let input_delegate_account = get_test_delegate_account_with_context();
        let delegate_amount = u64::MAX;
        let merkle_tree_index = 1;
        let epoch = 10;
        let forester_pda_pubkey = Pubkey::new_unique();

        let result = update_delegate_compressed_account::<false>(
            input_delegate_account.clone(),
            delegate_amount,
            merkle_tree_index,
            epoch,
            &forester_pda_pubkey,
        );

        assert!(result.is_err());
    }

    fn get_test_forester_account() -> ForesterAccount {
        ForesterAccount {
            active_stake_weight: 200,
            pending_undelegated_stake_weight: 50,
            ..Default::default()
        }
    }

    #[test]
    fn test_delegate_or_undelegate_delegate_pass() {
        let protocol_config = ProtocolConfig {
            ..Default::default()
        };
        let mut forester_pda = get_test_forester_account();
        let delegate_account = get_test_delegate_account_with_context();
        let authority = delegate_account.delegate_account.owner;
        let forester_pda_pubkey = delegate_account
            .delegate_account
            .delegate_forester_delegate_account
            .unwrap();
        let delegate_amount = 50;
        let current_slot = 10;
        let no_sync = true;

        let result = delegate_or_undelegate::<true>(
            &authority,
            &protocol_config,
            delegate_account,
            &forester_pda_pubkey,
            &mut forester_pda,
            delegate_amount,
            current_slot,
            no_sync,
        );

        let (input_delegate_pda, output_delegate_pda) = result.unwrap();
        assert_eq!(input_delegate_pda.compressed_account.owner, crate::ID);
        assert_eq!(output_delegate_pda.compressed_account.owner, crate::ID);

        let expected_delegate_account = DelegateAccount {
            delegated_stake_weight: delegate_account.delegate_account.delegated_stake_weight
                + delegate_amount,
            stake_weight: delegate_account.delegate_account.stake_weight - delegate_amount,
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
    }

    #[test]
    fn test_delegate_or_undelegate_undelegate_pass() {
        let protocol_config = ProtocolConfig {
            ..Default::default()
        };

        let mut forester_pda = get_test_forester_account();
        let delegate_account = get_test_delegate_account_with_context();
        let authority = delegate_account.delegate_account.owner;
        let forester_pda_pubkey = delegate_account
            .delegate_account
            .delegate_forester_delegate_account
            .unwrap();
        let delegate_amount = 50;
        let current_slot = 10;
        let no_sync = true;

        let result = delegate_or_undelegate::<false>(
            &authority,
            &protocol_config,
            delegate_account,
            &forester_pda_pubkey,
            &mut forester_pda,
            delegate_amount,
            current_slot,
            no_sync,
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
            pending_epoch: protocol_config.get_current_epoch(current_slot),
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
    }

    #[test]
    fn test_delegate_or_undelegate_undelegate_fail() {
        let authority = Pubkey::new_unique();
        let protocol_config = ProtocolConfig {
            ..Default::default()
        };
        let forester_pda_pubkey = Pubkey::new_unique();
        let mut forester_pda = get_test_forester_account();
        let delegate_account = get_test_delegate_account_with_context();
        let delegate_amount = u64::MAX;
        let current_slot = 10;
        let no_sync = true;

        let result = delegate_or_undelegate::<false>(
            &authority,
            &protocol_config,
            delegate_account,
            &forester_pda_pubkey,
            &mut forester_pda,
            delegate_amount,
            current_slot,
            no_sync,
        );

        assert!(matches!(result, Err(error) if error == RegistryError::InvalidAuthority.into()));
    }

    #[test]
    fn test_delegate_or_undelegate_delegate_fail() {
        let protocol_config = ProtocolConfig {
            ..Default::default()
        };
        let forester_pda_pubkey = Pubkey::new_unique();
        let mut forester_pda = get_test_forester_account();
        let delegate_account = get_test_delegate_account_with_context();
        let authority = delegate_account.delegate_account.owner;
        let delegate_amount = u64::MAX;
        let current_slot = 10;
        let no_sync = true;

        let result = delegate_or_undelegate::<true>(
            &authority,
            &protocol_config,
            delegate_account,
            &forester_pda_pubkey,
            &mut forester_pda,
            delegate_amount,
            current_slot,
            no_sync,
        );

        assert!(matches!(result, Err(error) if error == RegistryError::AlreadyDelegated.into()));
    }
}
