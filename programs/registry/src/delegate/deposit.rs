use crate::errors::RegistryError;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_compressed_token::{
    process_transfer::{InputTokenDataWithContext, PackedTokenTransferOutputData},
    TokenData,
};
use light_hasher::{errors::HasherError, DataHasher, Poseidon};
use light_system_program::sdk::compressed_account::{CompressedAccount, MerkleContext};
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        compressed_account::{
            CompressedAccountData, PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
        },
        CompressedCpiContext,
    },
    OutputCompressedAccountWithPackedContext,
};
use light_utils::hash_to_bn254_field_size_be;

use super::{
    deposit_instruction::DepositOrWithdrawInstruction,
    get_escrow_token_authority,
    process_cpi::{cpi_compressed_token_transfer, cpi_light_system_program},
    state::{DelegateAccount, InputDelegateAccount},
    DELEGATE_ACCOUNT_DISCRIMINATOR, ESCROW_TOKEN_ACCOUNT_SEED,
};

pub struct DepositCompressedAccounts {
    pub output_token_accounts: Vec<PackedTokenTransferOutputData>,
    pub input_delegate_pda: Option<PackedCompressedAccountWithMerkleContext>,
    pub output_delegate_pda: OutputCompressedAccountWithPackedContext,
}

pub fn process_deposit_or_withdrawal<'a, 'b, 'c, 'info: 'b + 'c, const IS_DEPOSIT: bool>(
    ctx: Context<'a, 'b, 'c, 'info, DepositOrWithdrawInstruction<'info>>,
    salt: u64,
    proof: CompressedProof,
    cpi_context: CompressedCpiContext,
    delegate_account: Option<InputDelegateAccountWithPackedContext>,
    deposit_amount: u64,
    mut input_compressed_token_accounts: Vec<InputTokenDataWithContext>,
    input_escrow_token_account: Option<InputTokenDataWithContext>,
    escrow_token_account_merkle_tree_index: u8,
    change_compressed_account_merkle_tree_index: u8,
    output_delegate_compressed_account_merkle_tree_index: u8,
) -> Result<()> {
    let mint = &ctx.accounts.protocol_config.config.mint;
    let slot = Clock::get()?.slot;
    let epoch = ctx.accounts.protocol_config.config.get_current_epoch(slot);
    let compressed_accounts = deposit_or_withdraw::<IS_DEPOSIT>(
        &ctx.accounts.authority.key(),
        &ctx.accounts.escrow_token_authority.key(),
        mint,
        delegate_account,
        deposit_amount,
        &input_compressed_token_accounts,
        &input_escrow_token_account,
        escrow_token_account_merkle_tree_index,
        change_compressed_account_merkle_tree_index,
        output_delegate_compressed_account_merkle_tree_index,
        epoch,
    )?;

    if let Some(input_escrow_token_account) = input_escrow_token_account {
        input_compressed_token_accounts.push(input_escrow_token_account);
    }
    let system_cpi_context = CompressedCpiContext {
        set_context: true,
        ..cpi_context
    };
    cpi_light_system_program(
        &ctx,
        None,
        Some(system_cpi_context),
        compressed_accounts.input_delegate_pda,
        compressed_accounts.output_delegate_pda,
        ctx.remaining_accounts.to_vec(),
    )?;
    let owner = ctx.accounts.authority.key();
    let (_, bump) = get_escrow_token_authority(&owner, salt);
    let bump = &[bump];
    let salt_bytes = salt.to_le_bytes();
    let seeds = [
        ESCROW_TOKEN_ACCOUNT_SEED,
        owner.as_ref(),
        salt_bytes.as_slice(),
        bump,
    ];
    let mut cpi_context = cpi_context;
    cpi_context.first_set_context = false;
    cpi_compressed_token_transfer(
        &ctx,
        Some(proof),
        None,
        false,
        salt,
        cpi_context,
        mint,
        input_compressed_token_accounts,
        compressed_accounts.output_token_accounts,
        &owner,
        ctx.accounts.escrow_token_authority.to_account_info(),
        seeds,
        ctx.remaining_accounts.to_vec(),
    )
}

// TODO: assert that escrow token account and delegate account sums match, all
// stakeweight or delegated tokens need to be in the escrow account
// TODO: require the token account to be last synced in the current epoch
// TODO: throw if inputs have a delegate
/// Deposit to a DelegateAccount
/// 1. Deposit compressed tokens to DelegatePda
///     inputs: InputTokenData, deposit_amount
///     create two outputs, escrow compressed account and change account
///     compressed escrow account is owned by pda derived from authority
pub fn deposit_or_withdraw<const IS_DEPOSIT: bool>(
    authority: &Pubkey,
    escrow_token_authority: &Pubkey,
    // get from ProtocolConfig
    mint: &Pubkey,
    // If None create new delegate account
    delegate_account: Option<InputDelegateAccountWithPackedContext>,
    deposit_amount: u64,
    input_compressed_token_accounts: &[InputTokenDataWithContext],
    // Input escrow token account is linked as its hash is part of the
    // DelegateAccount.
    input_escrow_token_account: &Option<InputTokenDataWithContext>,
    escrow_token_account_merkle_tree_index: u8,
    change_compressed_account_merkle_tree_index: u8,
    output_delegate_compressed_account_merkle_tree_index: u8,
    epoch: u64,
) -> Result<DepositCompressedAccounts> {
    if delegate_account.is_some() && input_escrow_token_account.is_none()
        || delegate_account.is_none() && input_escrow_token_account.is_some()
    {
        msg!("Delegate account and escrow token account must be provided together");
        return Err(RegistryError::InputEscrowTokenHashNotProvided.into());
    }
    if !IS_DEPOSIT && input_escrow_token_account.is_none() {
        msg!("An input compressed escrow token account is required for withdrawal");
        return Err(RegistryError::InputEscrowTokenHashNotProvided.into());
    }
    let hashed_owner = hash_to_bn254_field_size_be(authority.as_ref()).unwrap().0;
    let hashed_mint = hash_to_bn254_field_size_be(mint.as_ref()).unwrap().0;
    let hashed_escrow_token_authority =
        hash_to_bn254_field_size_be(escrow_token_authority.as_ref())
            .unwrap()
            .0;

    let sum_inputs = if IS_DEPOSIT {
        let sum_inputs = input_compressed_token_accounts
            .iter()
            .map(|x| x.amount)
            .sum::<u64>();
        if sum_inputs != deposit_amount {
            msg!(
                "Deposit amount does not match sum of input token accounts: {} != {}",
                deposit_amount,
                sum_inputs
            );
            return Err(RegistryError::DepositAmountNotEqualInputAmount.into());
        }
        sum_inputs
    } else {
        0
    };

    let output_escrow_token_account = update_escrow_compressed_token_account::<IS_DEPOSIT>(
        escrow_token_authority,
        input_escrow_token_account,
        deposit_amount,
        escrow_token_account_merkle_tree_index,
    )?;
    let input_escrow_token_account_hash =
        if let Some(input_escrow_token_account) = input_escrow_token_account.as_ref() {
            Some(
                hash_input_token_data_with_context(
                    &hashed_mint,
                    &hashed_owner,
                    input_escrow_token_account.amount,
                )
                .map_err(ProgramError::from)?,
            )
        } else {
            None
        };
    let output_bytes = output_escrow_token_account.amount.to_le_bytes();
    let output_compressed_token_hash = TokenData::hash_with_hashed_values::<Poseidon>(
        &hashed_mint,
        &hashed_escrow_token_authority,
        &output_bytes,
        &None,
    )
    .map_err(ProgramError::from)?;

    let mut output_token_accounts = Vec::new();
    output_token_accounts.push(output_escrow_token_account);

    if deposit_amount != sum_inputs {
        let change_compressed_token_account =
            create_change_output_compressed_token_account::<IS_DEPOSIT>(
                input_compressed_token_accounts,
                deposit_amount,
                authority,
                change_compressed_account_merkle_tree_index,
            )?;
        output_token_accounts.push(change_compressed_token_account);
    }
    // TODO: create a close account instruction
    let (input_delegate_pda, output_delegate_pda) = update_delegate_compressed_account::<IS_DEPOSIT>(
        delegate_account,
        authority,
        input_escrow_token_account_hash,
        output_compressed_token_hash,
        deposit_amount,
        output_delegate_compressed_account_merkle_tree_index,
        epoch,
    )?;
    Ok(DepositCompressedAccounts {
        input_delegate_pda,
        output_delegate_pda,
        output_token_accounts,
    })
}

#[derive(Clone, Debug, PartialEq, AnchorDeserialize, AnchorSerialize)]
pub struct InputDelegateAccountWithPackedContext {
    pub root_index: u16,
    pub merkle_context: PackedMerkleContext,
    pub delegate_account: InputDelegateAccount,
}
#[derive(Clone, Debug, PartialEq, AnchorDeserialize, AnchorSerialize)]
pub struct InputDelegateAccountWithContext {
    pub root_index: u16,
    pub merkle_context: MerkleContext,
    pub delegate_account: InputDelegateAccount,
}

#[derive(Clone, Copy, Debug, PartialEq, AnchorDeserialize, AnchorSerialize)]
pub struct DelegateAccountWithPackedContext {
    pub root_index: u16,
    pub merkle_context: PackedMerkleContext,
    pub delegate_account: DelegateAccount,
    pub output_merkle_tree_index: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, AnchorDeserialize, AnchorSerialize)]
pub struct DelegateAccountWithContext {
    pub merkle_context: MerkleContext,
    pub delegate_account: DelegateAccount,
    pub output_merkle_tree_index: Pubkey,
}

pub fn hash_input_token_data_with_context(
    mint: &[u8; 32],
    hashed_owner: &[u8; 32],
    amount: u64,
) -> std::result::Result<[u8; 32], HasherError> {
    let amount_bytes = amount.to_le_bytes();
    TokenData::hash_with_hashed_values::<Poseidon>(mint, hashed_owner, &amount_bytes, &None)
}

fn update_delegate_compressed_account<const IS_DEPOSIT: bool>(
    input_delegate_account: Option<InputDelegateAccountWithPackedContext>,
    authority: &Pubkey,
    input_escrow_token_account_hash: Option<[u8; 32]>,
    output_escrow_token_account_hash: [u8; 32],
    deposit_amount: u64,
    merkle_tree_index: u8,
    epoch: u64,
) -> Result<(
    Option<PackedCompressedAccountWithMerkleContext>,
    OutputCompressedAccountWithPackedContext,
)> {
    let (input_account, mut delegate_account) = if let Some(input) = input_delegate_account {
        let input_escrow_token_account_hash =
            if let Some(input_escrow_token_account_hash) = input_escrow_token_account_hash {
                Ok(input_escrow_token_account_hash)
            } else {
                err!(RegistryError::InputEscrowTokenHashNotProvided)
            }?;
        let (mut delegate_account, input_account) =
            create_input_delegate_account(authority, input_escrow_token_account_hash, input)?;
        delegate_account.escrow_token_account_hash = output_escrow_token_account_hash;
        (Some(input_account), delegate_account)
    } else {
        (
            None,
            DelegateAccount {
                owner: *authority,
                escrow_token_account_hash: output_escrow_token_account_hash,
                last_sync_epoch: epoch,
                ..Default::default()
            },
        )
    };
    if IS_DEPOSIT {
        delegate_account.stake_weight = delegate_account
            .stake_weight
            .checked_add(deposit_amount)
            .ok_or(RegistryError::ComputeEscrowAmountFailed)?;
    } else {
        delegate_account.stake_weight = delegate_account
            .stake_weight
            .checked_sub(deposit_amount)
            .ok_or(RegistryError::ComputeEscrowAmountFailed)?;
    }

    let output_account: CompressedAccount =
        create_delegate_compressed_account::<false>(&delegate_account)?;
    let output_account_with_merkle_context = OutputCompressedAccountWithPackedContext {
        compressed_account: output_account,
        merkle_tree_index,
    };
    Ok((input_account, output_account_with_merkle_context))
}

pub fn create_input_delegate_account(
    authority: &Pubkey,
    input_escrow_token_account_hash: [u8; 32],
    input: InputDelegateAccountWithPackedContext,
) -> Result<(DelegateAccount, PackedCompressedAccountWithMerkleContext)> {
    let delegate_account = DelegateAccount {
        owner: *authority,
        escrow_token_account_hash: input_escrow_token_account_hash,
        delegate_forester_delegate_account: input
            .delegate_account
            .delegate_forester_delegate_account,
        delegated_stake_weight: input.delegate_account.delegated_stake_weight,
        stake_weight: input.delegate_account.stake_weight,
        pending_epoch: input.delegate_account.pending_epoch,
        pending_undelegated_stake_weight: input.delegate_account.pending_undelegated_stake_weight,
        last_sync_epoch: input.delegate_account.last_sync_epoch,
        pending_token_amount: input.delegate_account.pending_token_amount,
        pending_synced_stake_weight: input.delegate_account.pending_synced_stake_weight,
        pending_delegated_stake_weight: input.delegate_account.pending_delegated_stake_weight,
    };
    let input_account = create_compressed_delegate_account(
        delegate_account,
        input.merkle_context,
        input.root_index,
    )?;
    Ok((delegate_account, input_account))
}

pub fn create_compressed_delegate_account(
    delegate_account: DelegateAccount,
    merkle_context: PackedMerkleContext,
    root_index: u16,
) -> Result<PackedCompressedAccountWithMerkleContext> {
    let compressed_account = create_delegate_compressed_account::<true>(&delegate_account)?;
    let input_account = PackedCompressedAccountWithMerkleContext {
        merkle_context,
        root_index,
        compressed_account,
    };
    Ok(input_account)
}

pub fn create_delegate_compressed_account<const IS_INPUT: bool>(
    delegate_account: &DelegateAccount,
) -> std::result::Result<CompressedAccount, Error> {
    let data = if IS_INPUT {
        Vec::new()
    } else {
        let mut data = Vec::with_capacity(DelegateAccount::LEN);

        DelegateAccount::serialize(delegate_account, &mut data).unwrap();
        data
    };
    let data_hash = delegate_account
        .hash::<Poseidon>()
        .map_err(ProgramError::from)?;
    let data = CompressedAccountData {
        discriminator: DELEGATE_ACCOUNT_DISCRIMINATOR,
        data_hash,
        data,
    };
    let output_account = CompressedAccount {
        owner: crate::ID,
        lamports: 0,
        address: None,
        data: Some(data),
    };
    Ok(output_account)
}

pub fn update_escrow_compressed_token_account<const IS_DEPOSIT: bool>(
    escrow_token_authority: &Pubkey,
    input_escrow_token_account: &Option<InputTokenDataWithContext>,
    amount: u64,
    merkle_tree_index: u8,
) -> Result<PackedTokenTransferOutputData> {
    let mut output_amount = if let Some(input_escrow_token_account) = input_escrow_token_account {
        input_escrow_token_account.amount
    } else {
        0
    };
    if IS_DEPOSIT {
        output_amount = output_amount
            .checked_add(amount)
            .ok_or(RegistryError::ComputeEscrowAmountFailed)?;
    } else {
        output_amount = output_amount
            .checked_sub(amount)
            .ok_or(RegistryError::ComputeEscrowAmountFailed)?;
    }
    Ok(PackedTokenTransferOutputData {
        amount: output_amount,
        owner: *escrow_token_authority,
        lamports: None,
        merkle_tree_index,
    })
}

fn create_change_output_compressed_token_account<const IS_DEPOSIT: bool>(
    input_token_data_with_context: &[InputTokenDataWithContext],
    deposit_amount: u64,
    owner: &Pubkey,
    merkle_tree_index: u8,
) -> Result<PackedTokenTransferOutputData> {
    let input_sum = input_token_data_with_context
        .iter()
        .map(|account| account.amount)
        .sum::<u64>();
    let change_amount = if IS_DEPOSIT {
        match input_sum.checked_sub(deposit_amount) {
            Some(change_amount) => Ok(change_amount),
            None => err!(RegistryError::ArithmeticUnderflow),
        }?
    } else {
        match input_sum.checked_add(deposit_amount) {
            Some(change_amount) => Ok(change_amount),
            None => err!(RegistryError::ArithmeticUnderflow),
        }?
    };
    Ok(PackedTokenTransferOutputData {
        amount: change_amount,
        owner: *owner,
        lamports: None,
        merkle_tree_index,
    })
}

#[cfg(test)]
mod tests {

    use light_compressed_token::token_data::AccountState;

    use super::*;

    fn get_input_token_data_with_context_test_data() -> Vec<InputTokenDataWithContext> {
        vec![
            InputTokenDataWithContext {
                amount: 100,
                delegate_index: Some(1),
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 0,
                    queue_index: None,
                },
                root_index: 0,
                lamports: Some(50),
            },
            InputTokenDataWithContext {
                amount: 50,
                delegate_index: Some(2),
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 0,
                    queue_index: None,
                },
                root_index: 0,
                lamports: Some(25),
            },
        ]
    }

    #[test]
    fn test_create_change_output_compressed_token_account_pass() {
        let input_token_data_with_context = get_input_token_data_with_context_test_data();
        let deposit_amount = 120;
        let owner = Pubkey::default();
        let merkle_tree_index = 0;

        let output = create_change_output_compressed_token_account::<true>(
            &input_token_data_with_context,
            deposit_amount,
            &owner,
            merkle_tree_index,
        )
        .unwrap();

        assert_eq!(output.amount, 30);
        assert_eq!(output.owner, owner);
        assert_eq!(output.merkle_tree_index, merkle_tree_index);
        assert_eq!(output.lamports, None);
    }

    #[test]
    fn test_create_change_output_compressed_token_account_fail() {
        let input_token_data_with_context = get_input_token_data_with_context_test_data();
        let deposit_amount = 200;
        let owner = Pubkey::default();
        let merkle_tree_index = 0;

        let res = create_change_output_compressed_token_account::<true>(
            &input_token_data_with_context,
            deposit_amount,
            &owner,
            merkle_tree_index,
        );
        assert!(matches!(
            res,
            Err(error) if error == RegistryError::ArithmeticUnderflow.into()
        ));
    }

    fn get_input_escrow_token_account(amount: u64) -> Option<InputTokenDataWithContext> {
        Some(InputTokenDataWithContext {
            amount,
            delegate_index: Some(1),
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 0,
                leaf_index: 0,
                queue_index: None,
            },
            root_index: 0,
            lamports: Some(50),
        })
    }

    #[test]
    fn test_update_escrow_compressed_token_account_deposit_pass() {
        let escrow_token_authority = Pubkey::default();
        let input_escrow_token_account = get_input_escrow_token_account(100);
        let amount = 50;
        let merkle_tree_index = 0;

        let output = update_escrow_compressed_token_account::<true>(
            &escrow_token_authority,
            &input_escrow_token_account,
            amount,
            merkle_tree_index,
        )
        .unwrap();

        assert_eq!(output.amount, 150);
        assert_eq!(output.owner, escrow_token_authority);
        assert_eq!(output.merkle_tree_index, merkle_tree_index);
        assert_eq!(output.lamports, None);
    }

    #[test]
    fn test_update_escrow_compressed_token_account_withdraw_pass() {
        let escrow_token_authority = Pubkey::default();
        let input_escrow_token_account = get_input_escrow_token_account(100);
        let amount = 50;
        let merkle_tree_index = 0;

        let output = update_escrow_compressed_token_account::<false>(
            &escrow_token_authority,
            &input_escrow_token_account,
            amount,
            merkle_tree_index,
        )
        .unwrap();

        assert_eq!(output.amount, 50);
        assert_eq!(output.owner, escrow_token_authority);
        assert_eq!(output.merkle_tree_index, merkle_tree_index);
        assert_eq!(output.lamports, None);
    }

    #[test]
    fn test_update_escrow_compressed_token_account_withdraw_fail() {
        let escrow_token_authority = Pubkey::default();
        let input_escrow_token_account = get_input_escrow_token_account(50);
        let amount = 100;
        let merkle_tree_index = 0;

        let res = update_escrow_compressed_token_account::<false>(
            &escrow_token_authority,
            &input_escrow_token_account,
            amount,
            merkle_tree_index,
        );
        assert!(matches!(
            res,
            Err(error) if error == RegistryError::ComputeEscrowAmountFailed.into()
        ));
    }

    #[test]
    fn test_update_escrow_compressed_token_account_deposit_fail() {
        let escrow_token_authority = Pubkey::default();
        let input_escrow_token_account = get_input_escrow_token_account(u64::MAX);
        let amount = 1;
        let merkle_tree_index = 0;

        let res = update_escrow_compressed_token_account::<true>(
            &escrow_token_authority,
            &input_escrow_token_account,
            amount,
            merkle_tree_index,
        );
        assert!(matches!(
            res,
            Err(error) if error == RegistryError::ComputeEscrowAmountFailed.into()
        ));
    }

    fn get_test_delegate_account() -> DelegateAccount {
        DelegateAccount {
            owner: Pubkey::new_unique(),
            delegate_forester_delegate_account: Some(Pubkey::new_unique()),
            delegated_stake_weight: 100,
            stake_weight: 200,
            pending_undelegated_stake_weight: 50,
            pending_epoch: 1,
            last_sync_epoch: 0,
            pending_token_amount: 25,
            escrow_token_account_hash: [0u8; 32],
            pending_synced_stake_weight: 0,
            pending_delegated_stake_weight: 0,
        }
    }

    #[test]
    fn test_create_delegate_compressed_account_pass() {
        let delegate_account = get_test_delegate_account();

        let result = create_delegate_compressed_account::<false>(&delegate_account);

        assert!(result.is_ok());

        let compressed_account = result.unwrap();
        assert_eq!(compressed_account.owner, crate::ID);
        assert_eq!(compressed_account.lamports, 0);
        assert!(compressed_account.address.is_none());
        assert!(compressed_account.data.is_some());

        let data = compressed_account.data.unwrap();
        assert_eq!(data.discriminator, DELEGATE_ACCOUNT_DISCRIMINATOR);
        assert_eq!(data.data_hash, delegate_account.hash::<Poseidon>().unwrap());

        let mut serialized_data = Vec::with_capacity(DelegateAccount::LEN);
        DelegateAccount::serialize(&delegate_account, &mut serialized_data).unwrap();
        assert_eq!(data.data, serialized_data);
    }
    fn get_test_input_delegate_account_with_context() -> InputDelegateAccountWithPackedContext {
        InputDelegateAccountWithPackedContext {
            delegate_account: InputDelegateAccount {
                delegate_forester_delegate_account: Some(Pubkey::new_unique()),
                delegated_stake_weight: 100,
                stake_weight: 100,
                pending_undelegated_stake_weight: 50,
                pending_epoch: 1,
                last_sync_epoch: 10,
                pending_token_amount: 25,
                pending_synced_stake_weight: 0,
                pending_delegated_stake_weight: 0,
            },
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                nullifier_queue_pubkey_index: 2,
                leaf_index: 3,
                queue_index: None,
            },
            root_index: 4,
        }
    }
    #[test]
    fn test_create_input_delegate_account_pass() {
        let authority = Pubkey::new_unique();
        let input_escrow_token_account_hash = [1u8; 32];
        let input = get_test_input_delegate_account_with_context();

        let result = create_input_delegate_account(
            &authority,
            input_escrow_token_account_hash,
            input.clone(),
        );

        assert!(result.is_ok());

        let (delegate_account, input_account) = result.unwrap();
        assert_eq!(delegate_account.owner, authority);
        assert_eq!(
            delegate_account.escrow_token_account_hash,
            input_escrow_token_account_hash
        );
        assert_eq!(
            delegate_account.delegate_forester_delegate_account,
            input.delegate_account.delegate_forester_delegate_account
        );
        assert_eq!(
            delegate_account.delegated_stake_weight,
            input.delegate_account.delegated_stake_weight
        );
        assert_eq!(
            delegate_account.stake_weight,
            input.delegate_account.stake_weight
        );
        assert_eq!(
            delegate_account.pending_epoch,
            input.delegate_account.pending_epoch
        );
        assert_eq!(
            delegate_account.pending_undelegated_stake_weight,
            input.delegate_account.pending_undelegated_stake_weight
        );
        assert_eq!(
            delegate_account.last_sync_epoch,
            input.delegate_account.last_sync_epoch
        );
        assert_eq!(
            delegate_account.pending_token_amount,
            input.delegate_account.pending_token_amount
        );

        let compressed_account = input_account.compressed_account;
        assert_eq!(compressed_account.owner, crate::ID);
        assert_eq!(compressed_account.lamports, 0);
        assert!(compressed_account.address.is_none());
        assert!(compressed_account.data.is_some());

        let data = compressed_account.data.unwrap();
        assert_eq!(data.discriminator, DELEGATE_ACCOUNT_DISCRIMINATOR);
        assert_eq!(data.data_hash, delegate_account.hash::<Poseidon>().unwrap());

        assert_eq!(input_account.merkle_context, input.merkle_context);
        assert_eq!(input_account.root_index, input.root_index);
    }

    #[test]
    fn test_update_delegate_compressed_account_pass() {
        let authority = Pubkey::new_unique();
        let input_escrow_token_account_hash = Some([1u8; 32]);
        let output_escrow_token_account_hash = [2u8; 32];
        let input = Some(get_test_input_delegate_account_with_context());
        let deposit_amount = 100;
        let merkle_tree_index = 11;

        let result = update_delegate_compressed_account::<true>(
            input.clone(),
            &authority,
            input_escrow_token_account_hash,
            output_escrow_token_account_hash,
            deposit_amount,
            merkle_tree_index,
            0,
        );

        assert!(result.is_ok());

        let (input_account, output_account_with_merkle_context) = result.unwrap();
        if let Some(input_account) = input_account.as_ref() {
            assert_eq!(input_account.root_index, 4);
            assert_eq!(input_account.merkle_context.merkle_tree_pubkey_index, 1);
            assert_eq!(input_account.merkle_context.nullifier_queue_pubkey_index, 2);
            assert_eq!(input_account.merkle_context.leaf_index, 3);
            assert_eq!(input_account.merkle_context.queue_index, None);
            let input_data = input_account.compressed_account.data.as_ref().unwrap();
            assert!(input_data.data.is_empty());
        }

        assert_eq!(
            output_account_with_merkle_context.merkle_tree_index,
            merkle_tree_index
        );
        let output_account = output_account_with_merkle_context
            .compressed_account
            .clone();
        assert_eq!(output_account.owner, crate::ID);
        assert_eq!(output_account.lamports, 0);
        assert!(output_account.address.is_none());
        assert!(output_account.data.is_some());

        let data = output_account.data.unwrap();
        assert_eq!(data.discriminator, DELEGATE_ACCOUNT_DISCRIMINATOR);
        assert_eq!(
            data.data_hash,
            output_account_with_merkle_context
                .compressed_account
                .data
                .as_ref()
                .unwrap()
                .data_hash
        );

        let output_delegate_account = DelegateAccount {
            owner: authority,
            escrow_token_account_hash: output_escrow_token_account_hash,
            delegate_forester_delegate_account: input
                .clone()
                .unwrap()
                .delegate_account
                .delegate_forester_delegate_account,
            delegated_stake_weight: input
                .clone()
                .unwrap()
                .delegate_account
                .delegated_stake_weight,
            stake_weight: input.clone().unwrap().delegate_account.stake_weight + deposit_amount,
            pending_undelegated_stake_weight: input
                .clone()
                .unwrap()
                .delegate_account
                .pending_undelegated_stake_weight,
            pending_epoch: input.clone().unwrap().delegate_account.pending_epoch,
            last_sync_epoch: input.clone().unwrap().delegate_account.last_sync_epoch,
            pending_token_amount: input.clone().unwrap().delegate_account.pending_token_amount,
            pending_synced_stake_weight: input
                .clone()
                .unwrap()
                .delegate_account
                .pending_synced_stake_weight,
            pending_delegated_stake_weight: input
                .clone()
                .unwrap()
                .delegate_account
                .pending_delegated_stake_weight,
        };
        // let mut serialized_data = Vec::with_capacity(DelegateAccount::LEN);
        let output_des = DelegateAccount::deserialize(&mut &data.data[..]).unwrap();
        assert_eq!(output_delegate_account, output_des);
    }

    fn get_test_input_token_data_with_context() -> InputTokenDataWithContext {
        InputTokenDataWithContext {
            amount: 100,
            delegate_index: None,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                nullifier_queue_pubkey_index: 2,
                leaf_index: 3,
                queue_index: None,
            },
            root_index: 5,
            lamports: Some(50),
        }
    }

    #[test]
    fn test_deposit_with_delegate_account() {
        let authority = Pubkey::new_unique();
        let escrow_token_authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let delegate_account = Some(get_test_input_delegate_account_with_context());
        let deposit_amount = 100;
        let input_compressed_token_accounts = vec![get_test_input_token_data_with_context()];
        let input_escrow_token_account = Some(get_test_input_token_data_with_context());
        assert_eq!(
            input_escrow_token_account.as_ref().unwrap().amount,
            delegate_account
                .as_ref()
                .unwrap()
                .delegate_account
                .stake_weight
        );
        let escrow_token_account_merkle_tree_index = 0;
        let change_compressed_account_merkle_tree_index = 1;
        let output_delegate_compressed_account_merkle_tree_index = 2;

        let result = deposit_or_withdraw::<true>(
            &authority,
            &escrow_token_authority,
            &mint,
            delegate_account.clone(),
            deposit_amount,
            &input_compressed_token_accounts,
            &input_escrow_token_account,
            escrow_token_account_merkle_tree_index,
            change_compressed_account_merkle_tree_index,
            output_delegate_compressed_account_merkle_tree_index,
            0,
        );

        assert!(result.is_ok());
        assert_deposit_or_withdraw_result::<true>(
            result.unwrap(),
            mint,
            authority,
            escrow_token_authority,
            delegate_account,
            deposit_amount,
            output_delegate_compressed_account_merkle_tree_index,
            &input_compressed_token_accounts,
            input_escrow_token_account,
        );
    }

    #[test]
    fn test_deposit_without_delegate_account() {
        let authority = Pubkey::new_unique();
        let escrow_token_authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let delegate_account = None;
        let deposit_amount = 100;
        let input_compressed_token_accounts = vec![get_test_input_token_data_with_context()];
        let input_escrow_token_account = None;
        let escrow_token_account_merkle_tree_index = 0;
        let change_compressed_account_merkle_tree_index = 1;
        let output_delegate_compressed_account_merkle_tree_index = 2;

        let result = deposit_or_withdraw::<true>(
            &authority,
            &escrow_token_authority,
            &mint,
            delegate_account.clone(),
            deposit_amount,
            &input_compressed_token_accounts,
            &input_escrow_token_account,
            escrow_token_account_merkle_tree_index,
            change_compressed_account_merkle_tree_index,
            output_delegate_compressed_account_merkle_tree_index,
            0,
        );

        assert!(result.is_ok());
        assert_deposit_or_withdraw_result::<true>(
            result.unwrap(),
            mint,
            authority,
            escrow_token_authority,
            delegate_account,
            deposit_amount,
            output_delegate_compressed_account_merkle_tree_index,
            &input_compressed_token_accounts,
            input_escrow_token_account,
        );
    }

    #[test]
    fn test_withdraw_with_delegate_account() {
        // Partial withdrawal
        {
            let authority = Pubkey::new_unique();
            let escrow_token_authority = Pubkey::new_unique();
            let mint = Pubkey::new_unique();
            let delegate_account = Some(get_test_input_delegate_account_with_context());
            let withdraw_amount = 100;
            let input_compressed_token_accounts = vec![];
            let input_escrow_token_account = Some(get_test_input_token_data_with_context());
            let escrow_token_account_merkle_tree_index = 0;
            let change_compressed_account_merkle_tree_index = 1;
            let output_delegate_compressed_account_merkle_tree_index = 2;

            let result = deposit_or_withdraw::<false>(
                &authority,
                &escrow_token_authority,
                &mint,
                delegate_account.clone(),
                withdraw_amount,
                &input_compressed_token_accounts,
                &input_escrow_token_account,
                escrow_token_account_merkle_tree_index,
                change_compressed_account_merkle_tree_index,
                output_delegate_compressed_account_merkle_tree_index,
                0,
            );

            assert!(result.is_ok());
            assert_deposit_or_withdraw_result::<false>(
                result.unwrap(),
                mint,
                authority,
                escrow_token_authority,
                delegate_account,
                withdraw_amount,
                output_delegate_compressed_account_merkle_tree_index,
                &input_compressed_token_accounts,
                input_escrow_token_account,
            );
        }
        // Full withdrawal
        {
            let authority = Pubkey::new_unique();
            let escrow_token_authority = Pubkey::new_unique();
            let mint = Pubkey::new_unique();
            let delegate_account = Some(get_test_input_delegate_account_with_context());
            let withdraw_amount = delegate_account
                .as_ref()
                .unwrap()
                .delegate_account
                .stake_weight;
            let input_compressed_token_accounts = vec![];
            let input_escrow_token_account = Some(get_test_input_token_data_with_context());
            let escrow_token_account_merkle_tree_index = 0;
            let change_compressed_account_merkle_tree_index = 1;
            let output_delegate_compressed_account_merkle_tree_index = 2;

            let result = deposit_or_withdraw::<false>(
                &authority,
                &escrow_token_authority,
                &mint,
                delegate_account.clone(),
                withdraw_amount,
                &input_compressed_token_accounts,
                &input_escrow_token_account,
                escrow_token_account_merkle_tree_index,
                change_compressed_account_merkle_tree_index,
                output_delegate_compressed_account_merkle_tree_index,
                0,
            );

            assert!(result.is_ok());
            assert_deposit_or_withdraw_result::<false>(
                result.unwrap(),
                mint,
                authority,
                escrow_token_authority,
                delegate_account,
                withdraw_amount,
                output_delegate_compressed_account_merkle_tree_index,
                &input_compressed_token_accounts,
                input_escrow_token_account,
            );
        }
    }

    #[test]
    fn test_withdraw_without_input_compressed_() {
        let authority = Pubkey::new_unique();
        let escrow_token_authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let delegate_account = Some(get_test_input_delegate_account_with_context());
        let withdraw_amount = 100;
        let input_compressed_token_accounts = vec![get_test_input_token_data_with_context()];
        let input_escrow_token_account = None;
        let escrow_token_account_merkle_tree_index = 0;
        let change_compressed_account_merkle_tree_index = 1;
        let output_delegate_compressed_account_merkle_tree_index = 2;

        let result = deposit_or_withdraw::<false>(
            &authority,
            &escrow_token_authority,
            &mint,
            delegate_account.clone(),
            withdraw_amount,
            &input_compressed_token_accounts,
            &input_escrow_token_account,
            escrow_token_account_merkle_tree_index,
            change_compressed_account_merkle_tree_index,
            output_delegate_compressed_account_merkle_tree_index,
            0,
        );

        assert!(matches!(
            result,
            Err(error) if error == RegistryError::InputEscrowTokenHashNotProvided.into()
        ));
    }

    fn assert_deposit_or_withdraw_result<const IS_DEPOSIT: bool>(
        result: DepositCompressedAccounts,
        mint: Pubkey,
        authority: Pubkey,
        escrow_token_authority: Pubkey,
        delegate_account: Option<InputDelegateAccountWithPackedContext>,
        amount: u64,
        output_delegate_compressed_account_merkle_tree_index: u8,
        input_compressed_token_accounts: &Vec<InputTokenDataWithContext>,
        input_token_escrow_account: Option<InputTokenDataWithContext>,
    ) {
        let input_sum = input_compressed_token_accounts
            .iter()
            .map(|x| x.amount)
            .sum::<u64>();
        let input_escrow_amount =
            if let Some(input_token_escrow_account) = input_token_escrow_account.as_ref() {
                input_token_escrow_account.amount
            } else {
                0
            };

        let expected_escrow_amount = if IS_DEPOSIT {
            input_escrow_amount + amount
        } else {
            input_escrow_amount - amount
        };

        let input_token_data = TokenData {
            mint,
            owner: authority,
            amount: input_escrow_amount,
            delegate: None,
            state: AccountState::Initialized,
        };
        let output_escrow_token_data = TokenData {
            mint,
            owner: escrow_token_authority,
            amount: expected_escrow_amount,
            delegate: None,
            state: AccountState::Initialized,
        };

        if IS_DEPOSIT {
            assert_eq!(result.output_token_accounts.len(), 1);
            assert_eq!(result.output_token_accounts[0].amount, input_sum - amount);
            assert_eq!(result.output_token_accounts[0].owner, authority);
            assert_eq!(result.output_token_accounts[0].merkle_tree_index, 1);
        } else {
            assert_eq!(result.output_token_accounts.len(), 1);
            assert_eq!(result.output_token_accounts[0].amount, input_sum + amount);
            assert_eq!(result.output_token_accounts[0].owner, authority);
            assert_eq!(result.output_token_accounts[0].merkle_tree_index, 1);
        }

        assert_eq!(
            result.output_token_accounts[1].amount,
            output_escrow_token_data.amount
        );
        assert_eq!(
            result.output_token_accounts[1].owner,
            escrow_token_authority
        );

        let expected_output_delegate_pda = if let Some(delegate_account) = delegate_account.as_ref()
        {
            let expected_input_delegate_pda = Some(PackedCompressedAccountWithMerkleContext {
                compressed_account: create_delegate_compressed_account::<true>(&DelegateAccount {
                    owner: authority,
                    escrow_token_account_hash: input_token_data.hash::<Poseidon>().unwrap(),
                    delegate_forester_delegate_account: delegate_account
                        .delegate_account
                        .delegate_forester_delegate_account,
                    delegated_stake_weight: delegate_account
                        .delegate_account
                        .delegated_stake_weight,
                    stake_weight: delegate_account.delegate_account.stake_weight,
                    pending_epoch: delegate_account.delegate_account.pending_epoch,
                    pending_undelegated_stake_weight: delegate_account
                        .delegate_account
                        .pending_undelegated_stake_weight,
                    last_sync_epoch: delegate_account.delegate_account.last_sync_epoch,
                    pending_token_amount: delegate_account.delegate_account.pending_token_amount,
                    pending_synced_stake_weight: delegate_account
                        .delegate_account
                        .pending_synced_stake_weight,
                    pending_delegated_stake_weight: delegate_account
                        .delegate_account
                        .pending_delegated_stake_weight,
                })
                .unwrap(),
                merkle_context: delegate_account.merkle_context,
                root_index: 4,
            });
            assert_eq!(result.input_delegate_pda, expected_input_delegate_pda);
            let stake_weight = if IS_DEPOSIT {
                delegate_account.delegate_account.stake_weight + amount
            } else {
                delegate_account.delegate_account.stake_weight - amount
            };
            assert_eq!(stake_weight, expected_escrow_amount);

            OutputCompressedAccountWithPackedContext {
                compressed_account: create_delegate_compressed_account::<false>(&DelegateAccount {
                    owner: authority,
                    escrow_token_account_hash: output_escrow_token_data.hash::<Poseidon>().unwrap(),
                    delegate_forester_delegate_account: delegate_account
                        .delegate_account
                        .delegate_forester_delegate_account,
                    delegated_stake_weight: delegate_account
                        .delegate_account
                        .delegated_stake_weight,
                    stake_weight,
                    pending_epoch: delegate_account.delegate_account.pending_epoch,
                    pending_undelegated_stake_weight: delegate_account
                        .delegate_account
                        .pending_undelegated_stake_weight,
                    last_sync_epoch: delegate_account.delegate_account.last_sync_epoch,
                    pending_token_amount: delegate_account.delegate_account.pending_token_amount,
                    pending_synced_stake_weight: delegate_account
                        .delegate_account
                        .pending_synced_stake_weight,
                    pending_delegated_stake_weight: delegate_account
                        .delegate_account
                        .pending_delegated_stake_weight,
                })
                .unwrap(),
                merkle_tree_index: output_delegate_compressed_account_merkle_tree_index,
            }
        } else {
            assert_eq!(amount, expected_escrow_amount);
            OutputCompressedAccountWithPackedContext {
                compressed_account: create_delegate_compressed_account::<false>(&DelegateAccount {
                    owner: authority,
                    escrow_token_account_hash: output_escrow_token_data.hash::<Poseidon>().unwrap(),
                    delegate_forester_delegate_account: None,
                    delegated_stake_weight: 0,
                    stake_weight: amount,
                    pending_epoch: 0,
                    pending_undelegated_stake_weight: 0,
                    last_sync_epoch: 0,
                    pending_token_amount: 0,
                    pending_synced_stake_weight: 0,
                    pending_delegated_stake_weight: 0,
                })
                .unwrap(),
                merkle_tree_index: output_delegate_compressed_account_merkle_tree_index,
            }
        };
        assert_eq!(result.output_delegate_pda, expected_output_delegate_pda);
    }
}
