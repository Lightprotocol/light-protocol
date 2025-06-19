use anchor_lang::prelude::*;
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
    LightDiscriminator, LightHasher,
};

#[event]
#[derive(Clone, Debug, Default, LightHasher, LightDiscriminator)]
pub struct MyTokenCompressedAccount {
    pub amount: u64,
    #[hash]
    pub owner: Pubkey,
}

pub fn process_create_compressed_account<'info>(
    cpi_accounts: CpiAccounts,
    proof: ValidityProof,
    address_tree_info: PackedAddressTreeInfo,
    output_tree_index: u8,
    amount: u64,
) -> Result<()> {
    let (address, address_seed) = derive_address(
        &[
            b"deposit",
            cpi_accounts.fee_payer().key().to_bytes().as_ref(),
        ],
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
    my_compressed_account.owner = cpi_accounts.fee_payer().key();

    let cpi_inputs = CpiInputs {
        proof,
        account_infos: Some(vec![my_compressed_account
            .to_account_info()
            .map_err(ProgramError::from)?]),
        new_addresses: Some(vec![new_address_params]),
        cpi_context: Some(CompressedCpiContext {
            set_context: false,
            first_set_context: false,
            cpi_context_account_index: (cpi_accounts.system_accounts_len() - 1) as u8, // TODO: confirm
        }),
        ..Default::default()
    };

    cpi_inputs
        .invoke_light_system_program(cpi_accounts)
        .map_err(ProgramError::from)?;

    Ok(())
}

pub fn deposit_tokens(
    cpi_accounts: &CpiAccounts,
    token_metas: Vec<TokenAccountMeta>,
    output_tree_index: u8,
    mint: Pubkey,
    recipient: Pubkey,
    amount: u64,
) -> Result<()> {
    let sender_account = CTokenAccount::new(
        mint,
        *cpi_accounts.fee_payer().key,
        token_metas,
        output_tree_index,
    );
    let transfer_inputs = TransferInputs {
        fee_payer: *cpi_accounts.fee_payer().key,
        sender_account,
        // No validity proof necessary we are just storing things in the cpi context.
        validity_proof: None.into(),
        recipient,
        tree_pubkeys: cpi_accounts.tree_pubkeys().unwrap(),
        config: Some(TransferConfig {
            cpi_context: Some(CompressedCpiContext {
                set_context: true,
                first_set_context: true,
                cpi_context_account_index: (cpi_accounts.system_accounts_len() - 1) as u8,
            }),
            ..Default::default()
        }),
        amount,
    };
    let instruction =
        light_compressed_token_sdk::instructions::transfer::instruction::transfer(transfer_inputs)
            .unwrap();
    // We can use the property that account infos don't have to be in order if you use
    // solana program invoke.
    let mut account_infos = cpi_accounts.account_infos().to_vec();
    account_infos.push(cpi_accounts.fee_payer().clone());
    anchor_lang::solana_program::program::invoke(&instruction, account_infos.as_slice())?;

    Ok(())
}

// For pinocchio we will need to build the accounts in oder
// The easiest is probably just pass the accounts multiple times since deserialization is zero copy.
// pub struct TransferInstruction<'info> {
//     pub fee_payer: AccountInfo<'info>,
//     pub authority: AccountInfo<'info>,
//     pub cpi_authority_pda: AccountInfo<'info>,
//     pub light_system_program: AccountInfo<'info>,
//     pub registered_program_pda: AccountInfo<'info>,
//     /// CHECK: (account compression program) when emitting event.
//     pub noop_program: AccountInfo<'info>,
//     /// CHECK: (different program) is used to cpi account compression program from light system program.
//     pub account_compression_authority: AccountInfo<'info>,
//     pub account_compression_program: AccountInfo<'info>,
//     /// CHECK:(system program) used to derive cpi_authority_pda and check that
//     /// this program is the signer of the cpi.
//     pub self_program: AccountInfo<'info>,
//     pub token_pool_pda: Option<AccountInfo<'info>>,
//     pub compress_or_decompress_token_account: Option<InterfaceAccount<'info>>,
//     pub token_program: Option<AccountInfo<'info>>,
//     pub system_program: AccountInfo<'info>,
// }
