use anchor_lang::{prelude::*, Bumps};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{instruction::Instruction, program::invoke_signed};

use crate::{
    address::NewAddressParamsPacked,
    compressed_account::{
        OutputCompressedAccountWithPackedContext, PackedCompressedAccountWithMerkleContext,
    },
    proof::CompressedProof,
    traits::{
        InvokeAccounts, InvokeCpiAccounts, InvokeCpiContextAccount, LightSystemAccount,
        SignerAccounts,
    },
    PROGRAM_ID_SYSTEM,
};

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompressedCpiContext {
    /// Is set by the program that is invoking the CPI to signal that is should
    /// set the cpi context.
    pub set_context: bool,
    /// Is set to wipe the cpi context since someone could have set it before
    /// with unrelated data.
    pub first_set_context: bool,
    /// Index of cpi context account in remaining accounts.
    pub cpi_context_account_index: u8,
}

#[derive(Debug, PartialEq, Default, Clone, BorshDeserialize, BorshSerialize)]
pub struct InstructionDataInvokeCpi {
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub cpi_context: Option<CompressedCpiContext>,
}

// TODO: Currently, this function is not used anywhere. Before revisiting it,
// it needs:
//
// - Proper documentation of cpi-context and how to use it in SDK.
// - Turning into a simple check.
//
// pub fn get_compressed_cpi_context_account<'info>(
//     ctx: &Context<
//         '_,
//         '_,
//         '_,
//         'info,
//         impl InvokeAccounts<'info>
//             + LightSystemAccount<'info>
//             + InvokeCpiAccounts<'info>
//             + SignerAccounts<'info>
//             + Bumps,
//     >,
//     compressed_cpi_context: &CompressedCpiContext,
// ) -> Result<AccountInfo<'info>> {
//     let cpi_context_account = ctx
//         .remaining_accounts
//         .get(compressed_cpi_context.cpi_context_account_index as usize)
//         .map(|account| account.to_account_info())
//         .ok_or_else(|| Error::from(CpiContextAccountUndefined))?;
//     Ok(cpi_context_account)
// }

#[inline(always)]
pub fn setup_cpi_accounts<'info>(
    ctx: &Context<
        '_,
        '_,
        '_,
        'info,
        impl InvokeAccounts<'info>
            + LightSystemAccount<'info>
            + InvokeCpiAccounts<'info>
            + SignerAccounts<'info>
            + InvokeCpiContextAccount<'info>
            + Bumps,
    >,
) -> (Vec<AccountInfo<'info>>, Vec<AccountMeta>) {
    // The trick for having `None` accounts is to pass program ID, see
    // https://github.com/coral-xyz/anchor/pull/2101
    let none_account_info = ctx.accounts.get_light_system_program().to_account_info();

    let (cpi_context_account_info, cpi_context_account_meta) =
        match ctx.accounts.get_cpi_context_account() {
            Some(acc) => (
                acc.to_account_info(),
                AccountMeta {
                    pubkey: acc.key(),
                    is_signer: false,
                    is_writable: true,
                },
            ),
            None => (
                none_account_info.clone(),
                AccountMeta {
                    pubkey: ctx.accounts.get_light_system_program().key(),
                    is_signer: false,
                    is_writable: false,
                },
            ),
        };

    let mut account_infos = vec![
        // fee_payer
        ctx.accounts.get_fee_payer().to_account_info(),
        // authority
        ctx.accounts.get_authority().to_account_info(),
        // registered_program_pda
        ctx.accounts.get_registered_program_pda().to_account_info(),
        // noop_program
        ctx.accounts.get_noop_program().to_account_info(),
        // account_compression_authority
        ctx.accounts
            .get_account_compression_authority()
            .to_account_info(),
        // account_compression_program
        ctx.accounts
            .get_account_compression_program()
            .to_account_info(),
        // invoking_program
        ctx.accounts.get_invoking_program().to_account_info(),
        // sol_pool_pda
        none_account_info.clone(),
        // decompression_recipient
        none_account_info,
        // system_program
        ctx.accounts.get_system_program().to_account_info(),
        // cpi_context_account
        cpi_context_account_info,
    ];
    for remaining_account in ctx.remaining_accounts {
        account_infos.push(remaining_account.to_owned());
    }

    let mut account_metas = vec![
        // fee_payer
        AccountMeta {
            pubkey: account_infos[0].key(),
            is_signer: true,
            is_writable: true,
        },
        // authority
        AccountMeta {
            pubkey: account_infos[1].key(),
            is_signer: true,
            is_writable: false,
        },
        // registered_program_pda
        AccountMeta {
            pubkey: account_infos[2].key(),
            is_signer: false,
            is_writable: false,
        },
        // noop_program
        AccountMeta {
            pubkey: account_infos[3].key(),
            is_signer: false,
            is_writable: false,
        },
        // account_compression_authority
        AccountMeta {
            pubkey: account_infos[4].key(),
            is_signer: false,
            is_writable: false,
        },
        // account_compression_program
        AccountMeta {
            pubkey: account_infos[5].key(),
            is_signer: false,
            is_writable: false,
        },
        // invoking_program
        AccountMeta {
            pubkey: account_infos[6].key(),
            is_signer: false,
            is_writable: false,
        },
        // sol_pool_pda
        AccountMeta {
            pubkey: account_infos[7].key(),
            is_signer: false,
            is_writable: false,
        },
        // decompression_recipient
        AccountMeta {
            pubkey: account_infos[8].key(),
            is_signer: false,
            is_writable: false,
        },
        // system_program
        AccountMeta {
            pubkey: account_infos[9].key(),
            is_signer: false,
            is_writable: false,
        },
        cpi_context_account_meta,
    ];
    for remaining_account in ctx.remaining_accounts {
        account_metas.extend(remaining_account.to_account_metas(None));
    }

    (account_infos, account_metas)
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct InvokeCpi {
    pub inputs: Vec<u8>,
}

#[inline(always)]
pub fn invoke_cpi(
    account_infos: &[AccountInfo],
    accounts_metas: Vec<AccountMeta>,
    inputs: Vec<u8>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let instruction_data = InvokeCpi { inputs };

    // `InvokeCpi`'s discriminator
    let mut data = [49, 212, 191, 129, 39, 194, 43, 196].to_vec();
    data.extend(instruction_data.try_to_vec()?);

    let instruction = Instruction {
        program_id: PROGRAM_ID_SYSTEM,
        accounts: accounts_metas,
        data,
    };
    invoke_signed(&instruction, account_infos, signer_seeds)?;

    Ok(())
}

/// Invokes the light system program to verify and apply a zk-compressed state
/// transition. Serializes CPI instruction data, configures necessary accounts,
/// and executes the CPI.
pub fn verify<'info, 'a, 'b, 'c>(
    ctx: &Context<
        '_,
        '_,
        '_,
        'info,
        impl InvokeAccounts<'info>
            + LightSystemAccount<'info>
            + InvokeCpiAccounts<'info>
            + SignerAccounts<'info>
            + InvokeCpiContextAccount<'info>
            + Bumps,
    >,
    inputs_struct: &InstructionDataInvokeCpi,
    signer_seeds: &'a [&'b [&'c [u8]]],
) -> Result<()> {
    let mut inputs: Vec<u8> = Vec::new();
    InstructionDataInvokeCpi::serialize(inputs_struct, &mut inputs).unwrap();

    let (account_infos, account_metas) = setup_cpi_accounts(ctx);
    invoke_cpi(&account_infos, account_metas, inputs, signer_seeds)?;
    Ok(())
}
