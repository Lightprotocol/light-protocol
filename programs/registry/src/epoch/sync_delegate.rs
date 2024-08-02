use crate::delegate::process_cpi::{
    approve_spl_token, cpi_compressed_token_transfer, get_cpi_signer_seeds,
};
use crate::delegate::process_deposit::{
    create_compressed_delegate_account, create_delegate_compressed_account,
    hash_input_token_data_with_context, update_escrow_compressed_token_account,
    DelegateAccountWithPackedContext,
};
use crate::delegate::{delegate_account::DelegateAccount, process_cpi::cpi_light_system_program};
use crate::delegate::{
    get_escrow_token_authority, ESCROW_TOKEN_ACCOUNT_SEED,
    FORESTER_EPOCH_RESULT_ACCOUNT_DISCRIMINATOR,
};
use crate::errors::RegistryError;
use crate::MINT;
use anchor_lang::prelude::*;
use light_compressed_token::process_transfer::{
    InputTokenDataWithContext, PackedTokenTransferOutputData,
};
use light_system_program::invoke::processor::CompressedProof;
use light_system_program::sdk::compressed_account::{
    CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
    PackedMerkleContext,
};
use light_system_program::sdk::CompressedCpiContext;
use light_system_program::OutputCompressedAccountWithPackedContext;
use light_utils::hash_to_bn254_field_size_be;
use num_traits::ToBytes;

use super::claim_forester::CompressedForesterEpochAccountInput;
use super::sync_delegate_instruction::SyncDelegateInstruction;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Copy, PartialEq)]
pub struct SyncDelegateTokenAccount {
    pub salt: u64,
    pub cpi_context: CompressedCpiContext,
}

/// THIS IS INSECURE
/// TODO: make secure by checking inclusion of the last compressed forester epoch pda
// 360 bytes +( 576 + 32 )accounts + 64 signature = 1032 bytes -> 200 bytes for 8
// CompressedForesterEpochAccountInput compressed accounts
pub fn process_sync_delegate_account<'info>(
    ctx: Context<'_, '_, '_, 'info, SyncDelegateInstruction<'info>>,
    delegate_account: DelegateAccountWithPackedContext, // 155 bytes
    previous_hash: [u8; 32],                            // 32 bytes
    compressed_forester_epoch_pdas: Vec<CompressedForesterEpochAccountInput>, // 4 bytes
    last_account_root_index: u16,                       // 2 bytes
    last_account_merkle_context: PackedMerkleContext,   // 7 bytes
    inclusion_proof: CompressedProof,                   // 128 bytes
    sync_delegate_token_account: Option<SyncDelegateTokenAccount>, // 12 bytes
    input_escrow_token_account: Option<InputTokenDataWithContext>, // 20 bytes
    output_token_account_merkle_tree_index: u8,
) -> Result<()> {
    let authority = ctx.accounts.authority.key();
    let escrow_authority = ctx
        .accounts
        .escrow_token_authority
        .as_ref()
        .map(|authority| authority.key());
    let slot = Clock::get()?.slot;
    let epoch = ctx
        .accounts
        .protocol_config
        .config
        .get_current_registration_epoch(slot);
    let (
        input_delegate_compressed_account,
        // TODO: we need readonly accounts just add a bool to input accounts context and don't nullify, skip in sum checks
        _input_readonly_compressed_forester_epoch_account,
        output_account_with_merkle_context,
        output_token_escrow_account,
    ) = sync_delegate_account_and_create_compressed_accounts(
        authority,
        delegate_account,
        compressed_forester_epoch_pdas,
        previous_hash,
        ctx.accounts.forester_pda.key(),
        last_account_merkle_context,
        last_account_root_index,
        &input_escrow_token_account,
        &escrow_authority,
        output_token_account_merkle_tree_index,
        epoch,
    )?;

    let cpi_context = if let Some(sync_delegate_token_account) = sync_delegate_token_account {
        let (salt, mut cpi_context) = (
            sync_delegate_token_account.salt,
            CompressedCpiContext {
                first_set_context: sync_delegate_token_account.cpi_context.first_set_context,
                set_context: true,
                ..sync_delegate_token_account.cpi_context
            },
        );
        let input_escrow_token_account =
            if let Some(input_escrow_token_account) = input_escrow_token_account {
                Ok(input_escrow_token_account)
            } else {
                err!(RegistryError::InvalidAuthority)
            }?;
        msg!(
            "input_escrow_token_account: {:?}",
            input_escrow_token_account
        );
        let output_token_escrow_account =
            if let Some(output_token_escrow_account) = output_token_escrow_account {
                Ok(output_token_escrow_account)
            } else {
                err!(RegistryError::InvalidAuthority)
            }?;
        msg!(
            "output_token_escrow_account: {:?}",
            output_token_escrow_account
        );
        let amount_diff = output_token_escrow_account.amount - input_escrow_token_account.amount;

        let cpi_signer = ctx
            .accounts
            .escrow_token_authority
            .as_ref()
            .unwrap()
            .to_account_info();
        msg!("cpi_signer: {:?}", cpi_signer.key());
        msg!(
            "forester_token_pool {:?}",
            ctx.accounts.forester_token_pool.as_ref().unwrap().key()
        );
        approve_spl_token(
            &ctx,
            amount_diff,
            ctx.accounts
                .forester_token_pool
                .as_ref()
                .unwrap()
                .to_account_info(),
            cpi_signer.to_account_info(),
            get_cpi_signer_seeds(),
        )?;

        let owner = ctx.accounts.authority.key();

        let mint = ctx.accounts.protocol_config.config.mint;
        let (_, bump) = get_escrow_token_authority(&owner, salt);
        let bump = &[bump];
        let salt_bytes = salt.to_le_bytes();
        let seeds = [
            ESCROW_TOKEN_ACCOUNT_SEED,
            owner.as_ref(),
            salt_bytes.as_slice(),
            bump,
        ];
        cpi_compressed_token_transfer(
            &ctx,
            None,
            Some(amount_diff),
            true,
            salt,
            cpi_context,
            &mint,
            vec![input_escrow_token_account],
            vec![output_token_escrow_account],
            &owner,
            cpi_signer,
            seeds,
            ctx.remaining_accounts.to_vec(),
        )?;
        cpi_context.set_context = sync_delegate_token_account.cpi_context.set_context;
        cpi_context.first_set_context = false;
        Some(cpi_context)
    } else {
        None
    };
    cpi_light_system_program(
        &ctx,
        Some(inclusion_proof),
        cpi_context,
        Some(input_delegate_compressed_account), // TODO: add readonly account
        output_account_with_merkle_context,
        ctx.remaining_accounts.to_vec(),
    )
}

fn sync_delegate_account_and_create_compressed_accounts(
    authority: Pubkey,
    mut delegate_account: DelegateAccountWithPackedContext,
    compressed_forester_epoch_pdas: Vec<CompressedForesterEpochAccountInput>,
    previous_hash: [u8; 32],
    forester_pda_pubkey: Pubkey,
    last_account_merkle_context: PackedMerkleContext,
    last_account_root_index: u16,
    input_escrow_token_account: &Option<InputTokenDataWithContext>,
    escrow_token_authority: &Option<Pubkey>,
    merkle_tree_index: u8,
    epoch: u64,
) -> Result<(
    PackedCompressedAccountWithMerkleContext,
    PackedCompressedAccountWithMerkleContext,
    OutputCompressedAccountWithPackedContext,
    Option<PackedTokenTransferOutputData>,
)> {
    if authority != delegate_account.delegate_account.owner {
        return err!(RegistryError::InvalidAuthority);
    }

    let input_delegate_compressed_account = create_compressed_delegate_account(
        delegate_account.delegate_account,
        delegate_account.merkle_context,
        delegate_account.root_index,
    )?;

    let last_forester_pda_hash = sync_delegate_account(
        &mut delegate_account.delegate_account,
        compressed_forester_epoch_pdas,
        previous_hash,
        forester_pda_pubkey,
    )?;
    let input_readonly_compressed_forester_epoch_account = create_compressed_forester_epoch_account(
        last_forester_pda_hash,
        last_account_merkle_context,
        last_account_root_index,
    );

    let output_escrow_account = if input_escrow_token_account.is_some() {
        let amount = delegate_account.delegate_account.pending_token_amount;
        let output_escrow_account = update_escrow_compressed_token_account::<true>(
            &escrow_token_authority.unwrap(),
            input_escrow_token_account,
            amount,
            merkle_tree_index,
        )?;
        delegate_account.delegate_account.pending_token_amount = 0;
        let hashed_owner = hash_to_bn254_field_size_be(escrow_token_authority.unwrap().as_ref())
            .unwrap()
            .0;
        let hashed_mint = hash_to_bn254_field_size_be(MINT.to_bytes().as_ref())
            .unwrap()
            .0;
        msg!("output_escrow_account: {:?}", output_escrow_account);
        let output_escrow_hash = hash_input_token_data_with_context(
            &hashed_mint,
            &hashed_owner,
            output_escrow_account.amount,
        );
        msg!("output_escrow_hash: {:?}", output_escrow_hash);
        delegate_account.delegate_account.escrow_token_account_hash = output_escrow_hash.unwrap();

        Some(output_escrow_account)
    } else {
        msg!("no escrow account");
        None
    };
    println!(
        "delegate_account.delegate_account {:?}",
        delegate_account.delegate_account
    );
    let output_account: CompressedAccount =
        create_delegate_compressed_account::<false>(&delegate_account.delegate_account)?;
    let output_account_with_merkle_context = OutputCompressedAccountWithPackedContext {
        compressed_account: output_account,
        merkle_tree_index: delegate_account.output_merkle_tree_index,
    };

    Ok((
        input_delegate_compressed_account,
        input_readonly_compressed_forester_epoch_account,
        output_account_with_merkle_context,
        output_escrow_account,
    ))
}

// TODO: test that hash is equivalent to manually instantiated account
fn create_compressed_forester_epoch_account(
    last_forester_pda_hash: [u8; 32],
    last_account_merkle_context: PackedMerkleContext,
    last_account_root_index: u16,
) -> PackedCompressedAccountWithMerkleContext {
    let data = CompressedAccountData {
        discriminator: FORESTER_EPOCH_RESULT_ACCOUNT_DISCRIMINATOR,
        data_hash: last_forester_pda_hash,
        data: Vec::new(),
    };
    let readonly_input_account = CompressedAccount {
        owner: crate::ID,
        lamports: 0,
        address: None,
        data: Some(data),
    };
    PackedCompressedAccountWithMerkleContext {
        compressed_account: readonly_input_account,
        merkle_context: last_account_merkle_context,
        root_index: last_account_root_index,
    }
}

/**
 * Issue:
 * - I can get into a situation where the registered epoch has been created,
 * delegate has claimed,
 * the claimed stake is not part of the current registered epoch yet
 * since it wasn't calculated until after registration has concluded
 * -> delegate has more stake than she should
 *
 * reproduces:
 * cargo test-sbf -p registry-test -- --test test_init --nocapture > output.txt 2>&1
 *
 * Ideas:
 * - don't allow claiming of the last epoch (leads to weird edge cases)
 * - have an extra field which holds the stake of the last epoch (doesn't count into active stake yet)
 *    - this seems to be more manageable but is also ugly
 *
 *
 * There are too many unknowns:
 * - what do I do if someone undelegates? (needs to sync completely first,then full undelegate should work,
 *  what about partial? -> we need to record the stake at the beginning of the last epoch that was synced and all subsequent epochs
 *   is this really an issue? this can only happen for one epoch because of the overlap of phases of different epochs -> lag between claiming
 *    - registration for epoch requires epoch pda which is always up to date of the latest claimed reward -> its really just 1 epoch reward
 *
 * -> I need a thorough offchain test to mock these scenarios
 *
 * This might take too much time, I need to:
 * - sleep
 * - switch to contention and make that prod ready first
 *
 */
// TODO: check whether we can simplify this logic
/// Sync Delegate Account:
/// - syncs the virtual balance of accumulated stake rewards to the stake
///   account
/// - it does not sync the token stake account balance
/// - the token stake account balance must be fully synced to perform any
///   actions that move delegated stake
/// 1. input a vector of compressed forester epoch accounts
/// 2. Check that epoch of first compressed forester epoch account is less than
///    DelegateAccount.last_sync_epoch
/// 3. iterate over all compressed forester epoch accounts, increase
///    Account.stake_weight by rewards_earned in every step
/// 4. set DelegateAccount.last_sync_epoch to the epoch of the last compressed
///    forester epoch account
/// 5. prove inclusion of last hash in State merkle tree (outside of this function)
pub fn sync_delegate_account(
    delegate_account: &mut DelegateAccount,
    compressed_forester_epoch_pdas: Vec<CompressedForesterEpochAccountInput>,
    // add last synced epoch to compressed_forester_epoch_pdas (probably need to add this to forester epoch pdas too)
    // keep rewards of none synced epochs in a vector, its active stake but not synced yet,
    // TODO: ensure that this pending stake can also be undelegated (there could be a case)
    mut previous_hash: [u8; 32],
    forester_pubkey: Pubkey,
) -> Result<[u8; 32]> {
    let last_sync_epoch = delegate_account.last_sync_epoch;
    if !compressed_forester_epoch_pdas.is_empty()
        && compressed_forester_epoch_pdas[0].epoch <= last_sync_epoch
        && last_sync_epoch != 0
    {
        return err!(RegistryError::StakeAccountAlreadySynced);
    }
    let hashed_forester_pubkey = hash_to_bn254_field_size_be(forester_pubkey.as_ref())
        .ok_or(RegistryError::HashToFieldError)?
        .0;
    let mut last_epoch = delegate_account.last_sync_epoch;
    for (i, compressed_forester_epoch_pda) in compressed_forester_epoch_pdas.iter().enumerate() {
        delegate_account.sync_pending_stake_weight(compressed_forester_epoch_pda.epoch);

        let compressed_forester_epoch_pda = compressed_forester_epoch_pda
            .into_compressed_forester_epoch_pda(previous_hash, crate::ID);
        previous_hash = compressed_forester_epoch_pda.hash(hashed_forester_pubkey)?;
        let pending_synced_stake_weight = if compressed_forester_epoch_pda.epoch - last_epoch == 1 {
            delegate_account.pending_synced_stake_weight
        } else {
            0
        };
        println!(
            "pending_synced_stake_weight: {:?}",
            pending_synced_stake_weight
        );
        println!(
            "pending_undelegated_stake_weight: {:?}",
            delegate_account.pending_undelegated_stake_weight
        );
        println!(
            "delegated_stake_weight: {:?}",
            delegate_account.delegated_stake_weight
        );
        println!(
            "total stake: {:?}",
            compressed_forester_epoch_pda.stake_weight
        );
        println!(
            "compressed_forester_epoch_pda.rewards_earned {:?}",
            compressed_forester_epoch_pda.rewards_earned
        );
        println!(
            "usable stake {:?}",
            delegate_account.delegated_stake_weight
                + delegate_account.pending_undelegated_stake_weight
                - pending_synced_stake_weight
        );
        // TODO: double check that this doesn't become an issue when undelegating
        let get_delegate_epoch_reward = compressed_forester_epoch_pda.get_reward(
            delegate_account.delegated_stake_weight
                + delegate_account.pending_undelegated_stake_weight
                - pending_synced_stake_weight,
        )?;
        println!("get_delegate_epoch_reward: {:?}", get_delegate_epoch_reward);
        delegate_account.delegated_stake_weight = delegate_account
            .delegated_stake_weight
            .checked_add(get_delegate_epoch_reward)
            .ok_or(RegistryError::ArithmeticOverflow)?;
        // Tokens are already minted and can be synced to the delegate account
        delegate_account.pending_token_amount = delegate_account
            .pending_token_amount
            .checked_add(get_delegate_epoch_reward)
            .ok_or(RegistryError::ArithmeticOverflow)?;
        // the registry account is always one epoch behind -> we need cache the
        // last epoch reward and subtract it when calculating the reward for the
        // next epoch
        delegate_account.pending_synced_stake_weight = get_delegate_epoch_reward;
        last_epoch = compressed_forester_epoch_pda.epoch;
        // last_stake_weight = compressed_forester_epoch_pda.stake_weight;
        // Return the last forester epoch account hash.
        // We need to prove inclusion of the last hash in the State merkle tree.
        if i == compressed_forester_epoch_pdas.len() - 1 {
            let last_delegate_account = compressed_forester_epoch_pda;
            delegate_account.last_sync_epoch = last_delegate_account.epoch;
            return Ok(previous_hash);
        }
    }
    // This error is unreachable since the loop returns after the last iteration.
    err!(RegistryError::StakeAccountSyncError)
}

#[cfg(test)]
mod tests {

    use std::f32::MIN;

    use crate::{
        delegate::DELEGATE_ACCOUNT_DISCRIMINATOR,
        epoch::claim_forester::CompressedForesterEpochAccount,
    };

    use super::*;
    use light_hasher::{DataHasher, Poseidon};
    use light_system_program::sdk::compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
        PackedMerkleContext,
    };

    fn get_test_data() -> (
        [u8; 32],
        PackedMerkleContext,
        u16,
        PackedCompressedAccountWithMerkleContext,
    ) {
        let last_forester_pda_hash = [1; 32];
        let last_account_merkle_context = PackedMerkleContext {
            merkle_tree_pubkey_index: 1,
            nullifier_queue_pubkey_index: 2,
            leaf_index: 1234,
            queue_index: None,
        };
        let last_account_root_index = 5;

        let expected_data = CompressedAccountData {
            discriminator: FORESTER_EPOCH_RESULT_ACCOUNT_DISCRIMINATOR,
            data_hash: last_forester_pda_hash,
            data: Vec::new(),
        };
        let expected_account = CompressedAccount {
            owner: crate::ID,
            lamports: 0,
            address: None,
            data: Some(expected_data),
        };
        let expected_output = PackedCompressedAccountWithMerkleContext {
            compressed_account: expected_account,
            merkle_context: last_account_merkle_context.clone(),
            root_index: last_account_root_index,
        };

        (
            last_forester_pda_hash,
            last_account_merkle_context,
            last_account_root_index,
            expected_output,
        )
    }

    #[test]
    fn test_create_compressed_forester_epoch_account_passing() {
        let (
            last_forester_pda_hash,
            last_account_merkle_context,
            last_account_root_index,
            expected_output,
        ) = get_test_data();

        let output = create_compressed_forester_epoch_account(
            last_forester_pda_hash,
            last_account_merkle_context,
            last_account_root_index,
        );

        assert_eq!(output, expected_output);
        let authority = Pubkey::new_unique();
        assert_eq!(
            output
                .compressed_account
                .hash::<Poseidon>(&authority, &output.merkle_context.leaf_index)
                .unwrap(),
            expected_output
                .compressed_account
                .hash::<Poseidon>(&authority, &expected_output.merkle_context.leaf_index)
                .unwrap(),
        );
    }

    #[test]
    fn test_create_compressed_forester_epoch_account_failing() {
        let (
            last_forester_pda_hash,
            last_account_merkle_context,
            last_account_root_index,
            mut expected_output,
        ) = get_test_data();
        expected_output
            .compressed_account
            .data
            .as_mut()
            .unwrap()
            .data_hash = [2; 32];
        let output = create_compressed_forester_epoch_account(
            last_forester_pda_hash,
            last_account_merkle_context,
            last_account_root_index,
        );
        assert_ne!(output, expected_output);
    }

    fn get_test_data_sync() -> (
        DelegateAccount,
        Vec<CompressedForesterEpochAccountInput>,
        [u8; 32],
        Pubkey,
        DelegateAccount,
    ) {
        let initial_delegate_account = DelegateAccount {
            owner: Pubkey::new_unique(),
            delegate_forester_delegate_account: None,
            delegated_stake_weight: 100,
            ..Default::default()
        };

        let compressed_forester_epoch_pdas = vec![
            CompressedForesterEpochAccountInput {
                rewards_earned: 10,
                epoch: 0,
                stake_weight: 100,
            },
            // Registry account for epoch 1 is inited when epoch 0 is still ongoing
            // -> rewards_earned are not yet calculated thus not included in stake_weight
            CompressedForesterEpochAccountInput {
                rewards_earned: 20,
                epoch: 1,
                stake_weight: 100,
            },
            CompressedForesterEpochAccountInput {
                rewards_earned: 30,
                epoch: 2,
                stake_weight: 110,
            },
        ];

        let previous_hash = [0; 32];
        let forester_pda_pubkey = Pubkey::new_unique();
        let mut expected_delegate_account = initial_delegate_account.clone();
        expected_delegate_account.delegated_stake_weight += 10 + 20 + 30;
        expected_delegate_account.pending_token_amount += 10 + 20 + 30;
        expected_delegate_account.last_sync_epoch = 2;
        expected_delegate_account.pending_synced_stake_weight = 30;
        (
            initial_delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
            expected_delegate_account,
        )
    }

    #[test]
    fn test_sync_delegate_account_passing() {
        let (
            mut delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
            expected_delegate_account,
        ) = get_test_data_sync();

        let result = sync_delegate_account(
            &mut delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
        );

        assert!(result.is_ok());
        assert_eq!(delegate_account, expected_delegate_account);
    }
    fn get_test_data_sync_inconcistent() -> (
        DelegateAccount,
        Vec<CompressedForesterEpochAccountInput>,
        [u8; 32],
        Pubkey,
        DelegateAccount,
    ) {
        let initial_delegate_account = DelegateAccount {
            owner: Pubkey::new_unique(),
            delegate_forester_delegate_account: None,
            delegated_stake_weight: 1000000,
            ..Default::default()
        };
        let rewards = 990000;

        let compressed_forester_epoch_pdas = vec![
            CompressedForesterEpochAccountInput {
                rewards_earned: rewards,
                epoch: 0,
                stake_weight: 1000000,
            },
            CompressedForesterEpochAccountInput {
                rewards_earned: rewards,
                epoch: 1,
                stake_weight: 1000000,
            },
            CompressedForesterEpochAccountInput {
                rewards_earned: rewards,
                epoch: 2,
                stake_weight: 1000000 + 990000,
            },
            // CompressedForesterEpochAccountInput {
            //     rewards_earned: 990000,
            //     epoch: 3,
            //     stake_weight: 1990000 + 2 * 990000,
            // },
        ];

        let previous_hash = [0; 32];
        let forester_pda_pubkey = Pubkey::new_unique();
        let mut expected_delegate_account = initial_delegate_account.clone();
        expected_delegate_account.delegated_stake_weight += 3 * rewards;
        expected_delegate_account.pending_token_amount += 3 * rewards;
        expected_delegate_account.last_sync_epoch = 2;
        expected_delegate_account.pending_synced_stake_weight = rewards;
        (
            initial_delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
            expected_delegate_account,
        )
    }
    #[test]
    fn test_sync_delegate_account_inconsistent_updates_passing() {
        let (
            mut delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
            expected_delegate_account,
        ) = get_test_data_sync_inconcistent();
        let result = sync_delegate_account(
            &mut delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
        );

        assert!(result.is_ok());
        assert_eq!(delegate_account, expected_delegate_account);
    }

    fn get_test_data_sync_skipped_epoch() -> (
        DelegateAccount,
        Vec<CompressedForesterEpochAccountInput>,
        [u8; 32],
        Pubkey,
        DelegateAccount,
    ) {
        let initial_delegate_account = DelegateAccount {
            owner: Pubkey::new_unique(),
            delegate_forester_delegate_account: None,
            delegated_stake_weight: 1000000,
            ..Default::default()
        };
        let rewards = 990000;

        let compressed_forester_epoch_pdas = vec![
            CompressedForesterEpochAccountInput {
                rewards_earned: rewards,
                epoch: 0,
                stake_weight: 1000000,
            },
            // CompressedForesterEpochAccountInput {
            //     rewards_earned: rewards,
            //     epoch: 1,
            //     stake_weight: 1000000,
            // },
            CompressedForesterEpochAccountInput {
                rewards_earned: rewards,
                epoch: 2,
                stake_weight: 1000000 + 990000,
            },
            // CompressedForesterEpochAccountInput {
            //     rewards_earned: 990000,
            //     epoch: 3,
            //     stake_weight: 1990000 + 2 * 990000,
            // },
        ];

        let previous_hash = [0; 32];
        let forester_pda_pubkey = Pubkey::new_unique();
        let mut expected_delegate_account = initial_delegate_account.clone();
        expected_delegate_account.delegated_stake_weight += 2 * rewards;
        expected_delegate_account.pending_token_amount += 2 * rewards;
        expected_delegate_account.last_sync_epoch = 2;
        expected_delegate_account.pending_synced_stake_weight = rewards;
        (
            initial_delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
            expected_delegate_account,
        )
    }

    #[test]
    fn test_sync_delegate_account_skipped_updates_passing() {
        let (
            mut delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
            expected_delegate_account,
        ) = get_test_data_sync_skipped_epoch();
        let result = sync_delegate_account(
            &mut delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
        );

        assert!(result.is_ok());
        assert_eq!(delegate_account, expected_delegate_account);
    }

    fn get_rnd_test_data() -> (
        DelegateAccount,
        Vec<CompressedForesterEpochAccountInput>,
        [u8; 32],
        Pubkey,
        DelegateAccount,
    ) {
        let initial_delegate_account = DelegateAccount {
            owner: Pubkey::new_unique(),
            delegate_forester_delegate_account: None,
            delegated_stake_weight: 1000000,
            ..Default::default()
        };
        let rewards = 990000;
        let num_iter = 2;
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        // let mut compressed_forester_epoch_pdas = vec![];
        let mut expected_rewards = 0;

        // for i in 0..num_iter {

        // }

        let compressed_forester_epoch_pdas = vec![
            CompressedForesterEpochAccountInput {
                rewards_earned: rewards,
                epoch: 0,
                stake_weight: 1000000,
            },
            // CompressedForesterEpochAccountInput {
            //     rewards_earned: rewards,
            //     epoch: 1,
            //     stake_weight: 1000000,
            // },
            CompressedForesterEpochAccountInput {
                rewards_earned: rewards,
                epoch: 2,
                stake_weight: 1000000 + 990000,
            },
            // CompressedForesterEpochAccountInput {
            //     rewards_earned: 990000,
            //     epoch: 3,
            //     stake_weight: 1990000 + 2 * 990000,
            // },
        ];

        let previous_hash = [0; 32];
        let forester_pda_pubkey = Pubkey::new_unique();
        let mut expected_delegate_account = initial_delegate_account.clone();
        expected_delegate_account.delegated_stake_weight += 2 * rewards;
        expected_delegate_account.pending_token_amount += 2 * rewards;
        expected_delegate_account.last_sync_epoch = 2;
        expected_delegate_account.pending_synced_stake_weight = rewards;
        (
            initial_delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
            expected_delegate_account,
        )
    }

    #[test]
    fn test_sync_delegate_account_undelegate_passing() {
        let (
            mut delegate_account,
            mut compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
            mut expected_delegate_account,
        ) = get_test_data_sync_inconcistent();
        // undelegate 50% in epoch 1 -> for the last epoch reward should only be 50%
        let undelegate = delegate_account.delegated_stake_weight / 2;

        delegate_account.pending_undelegated_stake_weight += undelegate;
        delegate_account.delegated_stake_weight -= undelegate;
        delegate_account.pending_epoch = 0;

        // third parties delegate additional stake in epoch 1 so that the delegates stake remains 50% even with rewards
        compressed_forester_epoch_pdas[2].stake_weight += 990000;

        expected_delegate_account.stake_weight += undelegate;
        expected_delegate_account.delegated_stake_weight -=
            undelegate + compressed_forester_epoch_pdas[0].rewards_earned;
        expected_delegate_account.pending_token_amount -=
            compressed_forester_epoch_pdas[0].rewards_earned;
        expected_delegate_account.pending_synced_stake_weight =
            compressed_forester_epoch_pdas[0].rewards_earned / 2;
        println!(
            "compressed_forester_epoch_pdas[0..2].to_vec() {:?}",
            compressed_forester_epoch_pdas[0..3].to_vec()
        );

        println!("pre delegate account {:?}", delegate_account);

        let result = sync_delegate_account(
            &mut delegate_account,
            compressed_forester_epoch_pdas[0..3].to_vec(),
            previous_hash,
            forester_pda_pubkey,
        );
        println!("undelegated undelegate : {:?}", undelegate);
        println!("post delegate account {:?}", delegate_account);

        assert!(result.is_ok());
        // println!(
        //     "{:?}",
        //     compressed_forester_epoch_pdas[0].rewards_earned
        //         - delegate_account.delegated_stake_weight
        // );
        assert_eq!(delegate_account, expected_delegate_account);
    }

    #[test]
    fn test_sync_delegate_account_failing() {
        let (
            mut delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
            mut expected_delegate_account,
        ) = get_test_data_sync();

        // Modify expected_delegate_account to be incorrect for the failing test
        expected_delegate_account.delegated_stake_weight -= 10;

        let result = sync_delegate_account(
            &mut delegate_account,
            compressed_forester_epoch_pdas,
            previous_hash,
            forester_pda_pubkey,
        );
        assert!(result.is_ok());
        assert_ne!(delegate_account, expected_delegate_account);
    }

    #[test]
    fn test_sync_delegate_account_and_create_compressed_accounts_no_token_sync_passing() {
        let authority = Pubkey::new_unique();
        let mut delegate_account = DelegateAccountWithPackedContext {
            root_index: 11,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                nullifier_queue_pubkey_index: 2,
                leaf_index: 1234,
                queue_index: None,
            },
            delegate_account: DelegateAccount {
                owner: authority,
                delegate_forester_delegate_account: None,
                delegated_stake_weight: 100,
                ..Default::default()
            },
            output_merkle_tree_index: 1,
        };
        let epoch = 0;
        let compressed_forester_epoch_pdas = vec![CompressedForesterEpochAccountInput {
            rewards_earned: 10,
            epoch,
            stake_weight: 100,
        }];
        let previous_hash = [1; 32];
        let forester_pda_pubkey = Pubkey::new_unique();
        let last_account_merkle_context = PackedMerkleContext {
            merkle_tree_pubkey_index: 1,
            nullifier_queue_pubkey_index: 2,
            leaf_index: 1234,
            queue_index: None,
        };
        let last_account_root_index = 5;
        let escrow_token_authority = Pubkey::new_unique();
        let merkle_tree_index = 1;
        let result = sync_delegate_account_and_create_compressed_accounts(
            authority,
            delegate_account,
            compressed_forester_epoch_pdas.clone(),
            previous_hash,
            forester_pda_pubkey,
            last_account_merkle_context,
            last_account_root_index,
            &None,
            &Some(escrow_token_authority),
            merkle_tree_index,
            epoch,
        );
        assert!(result.is_ok());
        let (
            input_delegate_compressed_account,
            input_readonly_compressed_forester_epoch_account,
            output_account_with_merkle_context,
            output_escrow_account,
        ) = result.unwrap();
        // delegate_account.delegate_account.pending_token_amount += 10;
        assert_eq!(
            input_delegate_compressed_account.merkle_context,
            delegate_account.merkle_context
        );
        assert_eq!(
            input_delegate_compressed_account.root_index,
            delegate_account.root_index
        );
        let data = CompressedAccountData {
            discriminator: DELEGATE_ACCOUNT_DISCRIMINATOR,
            data_hash: delegate_account
                .delegate_account
                .hash::<Poseidon>()
                .unwrap(),
            data: Vec::new(),
        };
        assert_eq!(
            input_delegate_compressed_account
                .compressed_account
                .data
                .unwrap(),
            data
        );

        let mut output_delegate_account = delegate_account.delegate_account.clone();
        let sum = compressed_forester_epoch_pdas
            .iter()
            .map(|x| x.rewards_earned)
            .sum::<u64>();
        output_delegate_account.delegated_stake_weight += sum;
        output_delegate_account.pending_token_amount += sum;
        output_delegate_account.pending_synced_stake_weight += compressed_forester_epoch_pdas
            .last()
            .unwrap()
            .rewards_earned;
        let mut data = Vec::new();
        output_delegate_account.serialize(&mut data).unwrap();

        let deserlized = DelegateAccount::deserialize_reader(
            &mut &output_account_with_merkle_context
                .compressed_account
                .data
                .as_ref()
                .unwrap()
                .data[..],
        )
        .unwrap();
        assert_eq!(output_delegate_account, deserlized);
        let data = CompressedAccountData {
            discriminator: DELEGATE_ACCOUNT_DISCRIMINATOR,
            data_hash: output_delegate_account.hash::<Poseidon>().unwrap(),
            data,
        };
        assert_eq!(
            output_account_with_merkle_context
                .compressed_account
                .data
                .unwrap(),
            data
        );
        assert_eq!(output_escrow_account, None);

        let ref_compressed_forester_epoch_account = CompressedForesterEpochAccount {
            rewards_earned: compressed_forester_epoch_pdas[0].rewards_earned,
            epoch: compressed_forester_epoch_pdas[0].epoch,
            stake_weight: compressed_forester_epoch_pdas[0].stake_weight,
            previous_hash,
            forester_pda_pubkey,
        };
        let hashed_forester_pubkey = hash_to_bn254_field_size_be(forester_pda_pubkey.as_ref())
            .unwrap()
            .0;

        let data = CompressedAccountData {
            discriminator: FORESTER_EPOCH_RESULT_ACCOUNT_DISCRIMINATOR,
            data_hash: ref_compressed_forester_epoch_account
                .hash(hashed_forester_pubkey)
                .unwrap(),
            data: Vec::new(),
        };
        assert_eq!(
            input_readonly_compressed_forester_epoch_account
                .compressed_account
                .data
                .unwrap(),
            data
        );
    }

    #[test]
    fn test_sync_delegate_account_and_create_compressed_accounts_with_token_sync_passing() {
        let authority = Pubkey::new_unique();
        let delegate_account = DelegateAccountWithPackedContext {
            root_index: 11,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                nullifier_queue_pubkey_index: 2,
                leaf_index: 1234,
                queue_index: None,
            },
            delegate_account: DelegateAccount {
                owner: authority,
                delegate_forester_delegate_account: None,
                delegated_stake_weight: 100,
                ..Default::default()
            },
            output_merkle_tree_index: 1,
        };
        let epoch = 0;
        let compressed_forester_epoch_pdas = vec![CompressedForesterEpochAccountInput {
            rewards_earned: 10,
            epoch,
            stake_weight: 100,
        }];
        let previous_hash = [1; 32];
        let forester_pda_pubkey = Pubkey::new_unique();
        let last_account_merkle_context = PackedMerkleContext {
            merkle_tree_pubkey_index: 1,
            nullifier_queue_pubkey_index: 2,
            leaf_index: 1234,
            queue_index: None,
        };
        let input_escrow_token_account = InputTokenDataWithContext {
            amount: 100,
            lamports: None,
            delegate_index: None,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                nullifier_queue_pubkey_index: 2,
                leaf_index: 1234,
                queue_index: None,
            },
            root_index: 5,
        };
        let last_account_root_index = 5;
        let escrow_token_authority = Pubkey::new_unique();
        let merkle_tree_index = 1;
        let result = sync_delegate_account_and_create_compressed_accounts(
            authority,
            delegate_account,
            compressed_forester_epoch_pdas.clone(),
            previous_hash,
            forester_pda_pubkey,
            last_account_merkle_context,
            last_account_root_index,
            &Some(input_escrow_token_account),
            &Some(escrow_token_authority),
            merkle_tree_index,
            epoch,
        );

        assert!(result.is_ok());
        let (
            input_delegate_compressed_account,
            input_readonly_compressed_forester_epoch_account,
            output_account_with_merkle_context,
            output_escrow_account,
        ) = result.unwrap();
        assert_eq!(
            input_delegate_compressed_account.merkle_context,
            delegate_account.merkle_context
        );
        assert_eq!(
            input_delegate_compressed_account.root_index,
            delegate_account.root_index
        );
        let data = CompressedAccountData {
            discriminator: DELEGATE_ACCOUNT_DISCRIMINATOR,
            data_hash: delegate_account
                .delegate_account
                .hash::<Poseidon>()
                .unwrap(),
            data: Vec::new(),
        };
        assert_eq!(
            input_delegate_compressed_account
                .compressed_account
                .data
                .unwrap(),
            data
        );

        let mut output_delegate_account = delegate_account.delegate_account.clone();
        let sum = compressed_forester_epoch_pdas
            .iter()
            .map(|x| x.rewards_earned)
            .sum::<u64>();
        output_delegate_account.delegated_stake_weight += sum;
        output_delegate_account.pending_synced_stake_weight = compressed_forester_epoch_pdas
            .last()
            .unwrap()
            .rewards_earned;
        output_delegate_account.pending_token_amount = 0;
        let hashed_escrow_owner = hash_to_bn254_field_size_be(escrow_token_authority.as_ref())
            .unwrap()
            .0;
        let hashed_bytes = hash_to_bn254_field_size_be(MINT.to_bytes().as_ref())
            .unwrap()
            .0;
        output_delegate_account.escrow_token_account_hash = hash_input_token_data_with_context(
            &hashed_bytes,
            &hashed_escrow_owner,
            output_escrow_account.unwrap().amount,
        )
        .unwrap();
        let mut data = Vec::new();
        output_delegate_account.serialize(&mut data).unwrap();
        let data = CompressedAccountData {
            discriminator: DELEGATE_ACCOUNT_DISCRIMINATOR,
            data_hash: output_delegate_account.hash::<Poseidon>().unwrap(),
            data,
        };
        let reader = DelegateAccount::deserialize_reader(
            &mut &output_account_with_merkle_context
                .compressed_account
                .data
                .as_ref()
                .unwrap()
                .data[..],
        )
        .unwrap();
        println!("reader {:?}", reader);
        assert_eq!(
            output_account_with_merkle_context
                .compressed_account
                .data
                .unwrap(),
            data
        );

        let ref_compressed_forester_epoch_account = CompressedForesterEpochAccount {
            rewards_earned: compressed_forester_epoch_pdas[0].rewards_earned,
            epoch: compressed_forester_epoch_pdas[0].epoch,
            stake_weight: compressed_forester_epoch_pdas[0].stake_weight,
            previous_hash,
            forester_pda_pubkey,
        };
        let hashed_forester_pubkey = hash_to_bn254_field_size_be(forester_pda_pubkey.as_ref())
            .unwrap()
            .0;

        let data = CompressedAccountData {
            discriminator: FORESTER_EPOCH_RESULT_ACCOUNT_DISCRIMINATOR,
            data_hash: ref_compressed_forester_epoch_account
                .hash(hashed_forester_pubkey)
                .unwrap(),
            data: Vec::new(),
        };
        assert_eq!(
            input_readonly_compressed_forester_epoch_account
                .compressed_account
                .data
                .unwrap(),
            data
        );
        let expected_output_escrow_account = PackedTokenTransferOutputData {
            owner: escrow_token_authority,
            lamports: None,
            amount: output_delegate_account.delegated_stake_weight,
            merkle_tree_index,
        };
        assert_eq!(output_escrow_account, Some(expected_output_escrow_account));
    }
}
