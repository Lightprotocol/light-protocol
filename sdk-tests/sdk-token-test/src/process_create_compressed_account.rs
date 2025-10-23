use anchor_lang::{prelude::*, solana_program::log::sol_log_compute_units};
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
    instruction::ValidityProof,
    light_account_checks::AccountInfoTrait,
    LightDiscriminator, LightHasher,
};

#[event]
#[derive(Clone, Debug, Default, LightHasher, LightDiscriminator)]
pub struct CompressedEscrowPda {
    pub amount: u64,
    #[hash]
    pub owner: Pubkey,
}

pub fn process_create_compressed_account<'a, 'info>(
    cpi_accounts: CpiAccounts<'a, 'info>,
    proof: ValidityProof,
    output_tree_index: u8,
    amount: u64,
    address: [u8; 32],
    new_address_params: light_sdk::address::NewAddressParamsAssignedPacked,
) -> Result<()> {
    let mut my_compressed_account =
        LightAccount::<CompressedEscrowPda>::new_init(&crate::ID, Some(address), output_tree_index);

    my_compressed_account.amount = amount;
    my_compressed_account.owner = *cpi_accounts.fee_payer().key;

    msg!("invoke");
    sol_log_compute_units();
    LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, proof)
        .with_light_account(my_compressed_account)?
        .with_new_addresses(&[new_address_params])
        .invoke(cpi_accounts)?;
    sol_log_compute_units();

    Ok(())
}

pub fn deposit_tokens<'a, 'info>(
    cpi_accounts: &CpiAccounts<'a, 'info>,
    token_metas: Vec<TokenAccountMeta>,
    output_tree_index: u8,
    mint: Pubkey,
    recipient: Pubkey,
    amount: u64,
    remaining_accounts: &[AccountInfo<'info>],
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
    let tree_account_infos = cpi_accounts.tree_accounts().unwrap();
    let tree_account_len = tree_account_infos.len();
    // skip cpi context account and omit the address tree and queue accounts.
    let tree_account_infos = &tree_account_infos[1..tree_account_len - 2];
    let tree_pubkeys = tree_account_infos
        .iter()
        .map(|x| x.pubkey())
        .collect::<Vec<Pubkey>>();
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
    // let account_infos: TransferAccountInfos<'_, 'info, MAX_ACCOUNT_INFOS> = TransferAccountInfos {
    //     fee_payer: cpi_accounts.fee_payer(),
    //     authority: cpi_accounts.fee_payer(),
    //     packed_accounts: tree_account_infos.as_slice(),
    //     ctoken_accounts: token_account_infos,
    //     cpi_context: Some(cpi_context),
    // };
    // let account_infos = account_infos.into_account_infos();
    // We can remove the address Merkle tree accounts.
    let len = remaining_accounts.len() - 2;
    // into_account_infos_checked() can be used for debugging but doubles CU cost to 1.5k CU
    let account_infos = [
        &[cpi_accounts.fee_payer().clone()][..],
        &remaining_accounts[..len],
    ]
    .concat();
    sol_log_compute_units();

    sol_log_compute_units();
    msg!("invoke");
    sol_log_compute_units();
    anchor_lang::solana_program::program::invoke(&instruction, account_infos.as_slice())?;
    sol_log_compute_units();

    Ok(())
}
