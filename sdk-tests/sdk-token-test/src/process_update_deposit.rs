use anchor_lang::prelude::*;
use light_batched_merkle_tree::queue::BatchedQueueAccount;
use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
use light_compressed_token_sdk::{
    account::CTokenAccount,
    instructions::transfer::instruction::{TransferConfig, TransferInputs},
    TokenAccountMeta,
};
use light_sdk::{
    account::LightAccount,
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    instruction::{PackedStateTreeInfo, ValidityProof},
    light_account_checks::AccountInfoTrait,
    LightDiscriminator, LightHasher,
};
use light_sdk_types::cpi_accounts::CpiAccountsConfig;

use crate::{PdaParams, TokenParams};

#[event]
#[derive(Clone, Debug, Default, LightHasher, LightDiscriminator)]
pub struct CompressedEscrowPda {
    pub amount: u64,
    #[hash]
    pub owner: Pubkey,
}

pub fn process_update_escrow_pda<'a, 'info>(
    cpi_accounts: CpiAccounts<'a, 'info>,
    pda_params: PdaParams,
    proof: ValidityProof,
    deposit_amount: u64,
) -> Result<()> {
    let mut my_compressed_account = LightAccount::<CompressedEscrowPda>::new_mut(
        &crate::ID,
        &pda_params.account_meta,
        CompressedEscrowPda {
            owner: *cpi_accounts.fee_payer().key,
            amount: pda_params.existing_amount,
        },
    )
    .unwrap();

    my_compressed_account.amount += deposit_amount;

    LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, proof)
        .with_light_account(my_compressed_account)?
        .invoke_execute_cpi_context(cpi_accounts)?;

    Ok(())
}

fn adjust_token_meta_indices(mut meta: TokenAccountMeta) -> TokenAccountMeta {
    meta.packed_tree_info.merkle_tree_pubkey_index -= 1;
    meta.packed_tree_info.queue_pubkey_index -= 1;
    meta
}

fn merge_escrow_token_accounts<'info>(
    tree_account_infos: Vec<AccountInfo<'info>>,
    fee_payer: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    mint: Pubkey,
    recipient: Pubkey,
    output_tree_queue_index: u8,
    escrowed_token_meta: TokenAccountMeta,
    escrow_token_account_meta_2: TokenAccountMeta,
    address: [u8; 32],
    recipient_bump: u8,
) -> Result<()> {
    // 3. Merge the newly escrowed tokens into the existing escrow account.
    // We remove the cpi context account -> we decrement all packed account indices by 1.
    let adjusted_queue_index = output_tree_queue_index - 1;
    let adjusted_escrowed_meta = adjust_token_meta_indices(escrowed_token_meta);
    let adjusted_escrow_meta_2 = adjust_token_meta_indices(escrow_token_account_meta_2);

    let escrow_account = CTokenAccount::new(
        mint,
        recipient,
        vec![adjusted_escrowed_meta, adjusted_escrow_meta_2],
        adjusted_queue_index,
    );

    let total_escrowed_amount = escrow_account.amount;

    let tree_pubkeys = tree_account_infos
        .iter()
        .map(|x| x.pubkey())
        .collect::<Vec<Pubkey>>();
    let transfer_inputs = TransferInputs {
        fee_payer: *fee_payer.key,
        sender_account: escrow_account,
        // No validity proof necessary we are just storing state in the cpi context.
        validity_proof: None.into(),
        recipient,
        tree_pubkeys,
        config: Some(TransferConfig {
            cpi_context: None,
            cpi_context_pubkey: None,
            ..Default::default()
        }),
        amount: total_escrowed_amount,
    };
    let instruction =
        light_compressed_token_sdk::instructions::transfer::instruction::transfer(transfer_inputs)
            .unwrap();

    let account_infos = [&[fee_payer, authority][..], remaining_accounts].concat();

    let seeds = [&b"escrow"[..], &address, &[recipient_bump]];
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        account_infos.as_slice(),
        &[&seeds],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_tokens_to_escrow_pda<'a, 'info>(
    cpi_accounts: &CpiAccounts<'a, 'info>,
    remaining_accounts: &[AccountInfo<'info>],
    mint: Pubkey,
    amount: u64,
    recipient: &Pubkey,
    output_tree_index: u8,
    output_tree_queue_index: u8,
    address: [u8; 32],
    recipient_bump: u8,
    depositing_token_metas: Vec<TokenAccountMeta>,
) -> Result<TokenAccountMeta> {
    // 1.transfer depositing token to recipient pda -> escrow token account 2
    let sender_account = CTokenAccount::new(
        mint,
        *cpi_accounts.fee_payer().key,
        depositing_token_metas,
        output_tree_queue_index,
    );
    // leaf index is the next index in the output queue,
    let output_queue = BatchedQueueAccount::output_from_account_info(
        cpi_accounts
            .get_tree_account_info(output_tree_queue_index as usize)
            .unwrap(),
    )
    .unwrap();
    // SAFETY: state trees are height 32 -> as u32 will always succeed
    let leaf_index = output_queue.batch_metadata.next_index as u32 + 1;

    let escrow_token_account_meta_2 = TokenAccountMeta {
        amount,
        delegate_index: None,
        lamports: None,
        tlv: None,
        packed_tree_info: PackedStateTreeInfo {
            root_index: 0, // not used proof by index
            prove_by_index: true,
            merkle_tree_pubkey_index: output_tree_index,
            queue_pubkey_index: output_tree_queue_index,
            leaf_index,
        },
    };

    // TODO: remove cpi context pda from tree accounts.
    // The confusing thing is that cpi context pda is the first packed account so it should be in the tree accounts.
    // because the tree accounts are packed accounts.
    // - rename tree_accounts to packed accounts
    // - omit cpi context in tree_pubkeys
    let tree_account_infos = cpi_accounts.tree_accounts().unwrap();
    let tree_account_infos = &tree_account_infos[1..];
    let tree_pubkeys = tree_account_infos
        .iter()
        .map(|x| x.pubkey())
        .collect::<Vec<Pubkey>>();
    let cpi_context_pubkey = *cpi_accounts.cpi_context().unwrap().key;
    let transfer_inputs = TransferInputs {
        fee_payer: *cpi_accounts.fee_payer().key,
        sender_account,
        // No validity proof necessary we are just storing state in the cpi context.
        validity_proof: None.into(),
        recipient: *recipient,
        tree_pubkeys,
        config: Some(TransferConfig {
            cpi_context: Some(CompressedCpiContext {
                set_context: true,
                first_set_context: true,
                // TODO: change to bool and add sanity check that if true account in index 0 is a cpi context pubkey
                cpi_context_account_index: 0, // TODO: replace with Pubkey (maybe not because it is in tree pubkeys 1 in this case)
            }),
            cpi_context_pubkey: Some(cpi_context_pubkey), // cpi context pubkey is in index 0.
            ..Default::default()
        }),
        amount,
    };
    let instruction =
        light_compressed_token_sdk::instructions::transfer::instruction::transfer(transfer_inputs)
            .unwrap();

    let account_infos = [&[cpi_accounts.fee_payer().clone()][..], remaining_accounts].concat();

    let seeds = [&b"escrow"[..], &address, &[recipient_bump]];
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        account_infos.as_slice(),
        &[&seeds],
    )?;

    Ok(escrow_token_account_meta_2)
}

pub fn process_update_deposit<'info>(
    ctx: Context<'_, '_, '_, 'info, crate::GenericWithAuthority<'info>>,
    output_tree_index: u8,
    output_tree_queue_index: u8,
    proof: ValidityProof,
    system_accounts_start_offset: u8,
    token_params: TokenParams,
    pda_params: PdaParams,
) -> Result<()> {
    // It makes sense to parse accounts once.
    let config = CpiAccountsConfig {
        cpi_signer: crate::LIGHT_CPI_SIGNER,
        cpi_context: true,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };

    let (_token_account_infos, system_account_infos) = ctx
        .remaining_accounts
        .split_at(system_accounts_start_offset as usize);
    // TODO: figure out why the offsets are wrong.
    // Could add with pre account infos Option<u8>
    let cpi_accounts =
        CpiAccounts::new_with_config(ctx.accounts.signer.as_ref(), system_account_infos, config);

    let recipient = *ctx.accounts.authority.key;
    // We want to keep only one escrow compressed token account
    // But ctoken transfers can only have one signer -> we cannot from 2 signers at the same time
    // 1. transfer depositing token to recipient pda -> escrow token account 2
    // 2. update escrow pda balance
    // 3. merge escrow token account 2 into escrow token account
    // Note:
    // - if the escrow pda only stores the amount and the owner we can omit the escrow pda.
    // - the escrowed token accounts are owned by a pda derived from the owner
    //      that is sufficient to verify ownership.
    // - no escrow pda will simplify the transaction, for no cpi context account is required
    let address = pda_params.account_meta.address;

    // 1.transfer depositing token to recipient pda -> escrow token account 2
    let escrow_token_account_meta_2 = transfer_tokens_to_escrow_pda(
        &cpi_accounts,
        ctx.remaining_accounts,
        token_params.mint,
        token_params.deposit_amount,
        &recipient,
        output_tree_index,
        output_tree_queue_index,
        address,
        token_params.recipient_bump,
        token_params.depositing_token_metas,
    )?;
    let tree_account_infos = cpi_accounts.tree_accounts().unwrap()[1..].to_vec();
    let fee_payer = cpi_accounts.fee_payer().clone();

    // 2. Update escrow pda balance
    // - settle tx 1 in the same instruction with the cpi context account
    process_update_escrow_pda(cpi_accounts, pda_params, proof, token_params.deposit_amount)?;

    // 3. Merge the newly escrowed tokens into the existing escrow account.
    merge_escrow_token_accounts(
        tree_account_infos,
        fee_payer,
        ctx.accounts.authority.to_account_info(),
        ctx.remaining_accounts,
        token_params.mint,
        recipient,
        output_tree_queue_index,
        token_params.escrowed_token_meta,
        escrow_token_account_meta_2,
        address,
        token_params.recipient_bump,
    )?;
    Ok(())
}
