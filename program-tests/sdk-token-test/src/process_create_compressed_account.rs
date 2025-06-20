use anchor_lang::prelude::*;
use anchor_lang::solana_program::log::sol_log_compute_units;
use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
use light_compressed_token_sdk::{
    account::CTokenAccount,
    instructions::transfer::{
        account_infos::{filter_packed_accounts, TransferAccountInfos, MAX_ACCOUNT_INFOS},
        instruction::{TransferConfig, TransferInputs},
    },
    TokenAccountMeta,
};
use light_sdk::{
    account::LightAccount,
    address::v1::derive_address,
    cpi::{CpiAccounts, CpiInputs},
    instruction::{PackedAddressTreeInfo, ValidityProof},
    light_account_checks::AccountInfoTrait,
    LightDiscriminator, LightHasher,
};

#[event]
#[derive(Clone, Debug, Default, LightHasher, LightDiscriminator)]
pub struct MyTokenCompressedAccount {
    pub amount: u64,
    #[hash]
    pub owner: Pubkey,
}

pub fn process_create_compressed_account(
    cpi_accounts: CpiAccounts,
    proof: ValidityProof,
    address_tree_info: PackedAddressTreeInfo,
    output_tree_index: u8,
    amount: u64,
) -> Result<()> {
    let (address, address_seed) = derive_address(
        &[b"deposit", cpi_accounts.fee_payer().key.to_bytes().as_ref()],
        &address_tree_info
            .get_tree_pubkey(&cpi_accounts)
            .map_err(|_| ErrorCode::AccountNotEnoughKeys)?,
        &crate::ID,
    );
    let new_address_params = address_tree_info.into_new_address_params_packed(address_seed);

    let mut my_compressed_account = LightAccount::<'_, MyTokenCompressedAccount>::new_init(
        &crate::ID,
        Some(address),
        output_tree_index,
    );

    my_compressed_account.amount = amount;
    my_compressed_account.owner = *cpi_accounts.fee_payer().key;

    let cpi_inputs = CpiInputs {
        proof,
        account_infos: Some(vec![my_compressed_account
            .to_account_info()
            .map_err(ProgramError::from)?]),
        new_addresses: Some(vec![new_address_params]),
        cpi_context: Some(CompressedCpiContext {
            set_context: false,
            first_set_context: false,
            cpi_context_account_index: 0, // seems to be useless. Seems to be unused.
                                          // TODO: unify the account meta generation on and offchain.
        }),
        ..Default::default()
    };
    msg!("invoke");
    sol_log_compute_units();
    cpi_inputs
        .invoke_light_system_program(cpi_accounts)
        .map_err(ProgramError::from)?;
    sol_log_compute_units();

    Ok(())
}

pub fn deposit_tokens<'info>(
    cpi_accounts: &CpiAccounts<'_, 'info>,
    token_metas: Vec<TokenAccountMeta>,
    output_tree_index: u8,
    mint: Pubkey,
    recipient: Pubkey,
    amount: u64,
    token_account_infos: &[AccountInfo<'info>],
) -> Result<()> {
    let sender_account = CTokenAccount::new(
        mint,
        *cpi_accounts.fee_payer().key,
        token_metas,
        output_tree_index,
    );

    // We need to be careful what accounts we pass.
    // Big accounts cost many CU.
    // TODO: replace
    let tree_account_infos =
        filter_packed_accounts(&[&sender_account], cpi_accounts.tree_accounts().unwrap());
    let tree_pubkeys = tree_account_infos
        .iter()
        .map(|x| x.pubkey())
        .collect::<Vec<Pubkey>>();
    // msg!("tree_pubkeys {:?}", tree_pubkeys);
    let cpi_context = cpi_accounts.cpi_context().unwrap();
    let cpi_context_pubkey = *cpi_accounts.cpi_context().unwrap().key;
    // msg!("cpi_context_pubkey {:?}", cpi_context_pubkey);
    let transfer_inputs = TransferInputs {
        fee_payer: *cpi_accounts.fee_payer().key,
        sender_account,
        // No validity proof necessary we are just storing state in the cpi context.
        validity_proof: None.into(),
        recipient,
        tree_pubkeys,
        config: Some(TransferConfig {
            cpi_context: Some(CompressedCpiContext {
                set_context: true,
                first_set_context: true,
                cpi_context_account_index: 0, // TODO: replace with Pubkey (maybe not because it is in tree pubkeys 1 in this case)
            }),
            cpi_context_pubkey: Some(cpi_context_pubkey),
            ..Default::default()
        }),
        amount,
    };
    let instruction =
        light_compressed_token_sdk::instructions::transfer::instruction::transfer(transfer_inputs)
            .unwrap();
    // msg!("instruction {:?}", instruction);
    // We can use the property that account infos don't have to be in order if you use
    // solana program invoke.
    sol_log_compute_units();

    msg!("create_account_infos");
    sol_log_compute_units();
    // TODO: initialize from CpiAccounts, use with_compressed_pda() offchain.
    let account_infos: TransferAccountInfos<'_, 'info, MAX_ACCOUNT_INFOS> = TransferAccountInfos {
        fee_payer: cpi_accounts.fee_payer(),
        authority: cpi_accounts.fee_payer(),
        packed_accounts: tree_account_infos.as_slice(),
        ctoken_accounts: token_account_infos,
        cpi_context: Some(cpi_context),
    };
    let account_infos = account_infos.into_account_infos();
    // into_account_infos_checked() can be used for debugging but doubles CU cost to 1.5k CU

    sol_log_compute_units();

    sol_log_compute_units();
    msg!("invoke");
    sol_log_compute_units();
    anchor_lang::solana_program::program::invoke(&instruction, account_infos.as_slice())?;
    sol_log_compute_units();

    Ok(())
}
