use anchor_lang::prelude::*;
use anchor_lang::solana_program::log::sol_log_compute_units;
use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
use light_compressed_token_sdk::{
    account::CTokenAccount,
    instructions::transfer::instruction::{TransferConfig, TransferInputs},
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
    // TODO: add a filter tree accounts function.
    let tree_account_infos =
        filter_tree_accounts(&[&sender_account], cpi_accounts.tree_accounts().unwrap());
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
    let account_infos = TransferAccountInfos {
        fee_payer: cpi_accounts.fee_payer(),
        authority: cpi_accounts.fee_payer(),
        tree_accounts: tree_account_infos.as_slice(),
        ctoken_accounts: token_account_infos,
        cpi_context: Some(cpi_context),
    };
    let account_infos = account_infos.into_account_infos();
    // 1390968
    // let account_infos = create_account_infos(
    //     &instruction,
    //     token_account_infos,
    //     &[cpi_accounts.fee_payer()],
    // );
    sol_log_compute_units();

    sol_log_compute_units();
    msg!("invoke");
    sol_log_compute_units();
    anchor_lang::solana_program::program::invoke(&instruction, account_infos.as_slice())?;
    sol_log_compute_units();

    Ok(())
}

// 7479
fn create_account_infos<'info>(
    instruction: &Instruction,
    account_infos: &[AccountInfo<'info>],
    additionals: &[&AccountInfo<'info>],
) -> Vec<AccountInfo<'info>> {
    let mut res_account_infos = Vec::with_capacity(instruction.accounts.len());

    for (i, account_meta) in instruction.accounts.iter().enumerate() {
        if let Some(account_info) = account_infos[i..]
            .iter()
            .find(|x| *x.key == account_meta.pubkey)
        {
            res_account_infos.push(account_info.clone());
        } else {
            if let Some(account_info) = additionals.iter().find(|x| *x.key == account_meta.pubkey) {
                res_account_infos.push((*account_info).clone());
            } else {
                if let Some(account_info) = account_infos[..i]
                    .iter()
                    .find(|x| *x.key == account_meta.pubkey)
                {
                    res_account_infos.push(account_info.clone());
                } else {
                    panic!("account not found");
                }
            }
        }
    }
    res_account_infos
}

// For pinocchio we will need to build the accounts in oder
// The easiest is probably just pass the accounts multiple times since deserialization is zero copy.
pub struct TransferAccountInfos<'a, 'info> {
    fee_payer: &'a AccountInfo<'info>,
    authority: &'a AccountInfo<'info>,
    ctoken_accounts: &'a [AccountInfo<'info>],
    cpi_context: Option<&'a AccountInfo<'info>>,
    // TODO: rename tree accounts to packed accounts
    tree_accounts: &'a [AccountInfo<'info>],
}

use anchor_lang::solana_program::instruction::Instruction;
impl<'info> TransferAccountInfos<'_, 'info> {
    // 874
    fn into_account_infos(self) -> Vec<AccountInfo<'info>> {
        // TODO: experiment with array vec.
        // we can use array vec with default constant say 20 and in case it's not enough
        // we throw an error that the constant needs to be increased.
        let mut capacity = 2 + self.ctoken_accounts.len() + self.tree_accounts.len();
        let ctoken_program_id_index = self.ctoken_accounts.len() - 2;
        if self.cpi_context.is_some() {
            capacity += 1;
        }
        let mut account_infos = Vec::with_capacity(capacity);
        account_infos.push(self.fee_payer.clone());
        account_infos.push(self.authority.clone());

        account_infos.extend_from_slice(self.ctoken_accounts);
        if let Some(cpi_context) = self.cpi_context {
            account_infos.push(cpi_context.clone());
        } else {
            account_infos.push(self.ctoken_accounts[ctoken_program_id_index].clone());
        }
        account_infos.extend_from_slice(self.tree_accounts);
        account_infos
    }

    // 1528
    fn into_account_infos_checked(self, ix: &Instruction) -> Result<Vec<AccountInfo<'info>>> {
        let account_infos = self.into_account_infos();
        for (account_meta, account_info) in ix.accounts.iter().zip(account_infos.iter()) {
            if account_meta.pubkey != *account_info.key {
                msg!("account meta {:?}", account_meta);
                msg!("account info {:?}", account_info);

                msg!("account metas {:?}", ix.accounts);
                msg!("account infos {:?}", account_infos);
                panic!("account info and meta don't match.");
            }
        }
        Ok(account_infos)
    }
}

// TODO: test
fn filter_tree_accounts<'info>(
    token_accounts: &[&CTokenAccount],
    account_infos: &[AccountInfo<'info>],
) -> Vec<AccountInfo<'info>> {
    let mut selected_account_infos = Vec::with_capacity(account_infos.len());
    account_infos
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            let i = *i as u8;
            token_accounts.iter().any(|y| {
                y.merkle_tree_index == i
                    || y.input_metas().iter().any(|z| {
                        z.packed_tree_info.merkle_tree_pubkey_index == i
                            || z.packed_tree_info.queue_pubkey_index == i
                            || {
                                if let Some(delegate_index) = z.delegate_index {
                                    delegate_index == i
                                } else {
                                    false
                                }
                            }
                    })
            })
        })
        .for_each(|x| selected_account_infos.push(x.1.clone()));
    selected_account_infos
}
