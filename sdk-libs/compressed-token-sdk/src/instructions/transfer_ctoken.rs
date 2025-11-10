use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// Create a c-token transfer instruction.
///
/// # Arguments
/// * `source` - Source token account
/// * `destination` - Destination token account
/// * `amount` - Amount to transfer
/// * `authority` - Authority pubkey
///
/// # Returns
/// `Instruction`
fn create_transfer_ctoken_instruction(
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
    authority: Pubkey,
) -> Instruction {
    Instruction {
        program_id: Pubkey::from(C_TOKEN_PROGRAM_ID),
        accounts: vec![
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: {
            let mut data = vec![3u8];
            data.push(3u8);
            data.extend_from_slice(&amount.to_le_bytes());
            data
        },
    }
}

/// Transfer c-tokens
pub fn transfer_ctoken<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
) -> Result<(), ProgramError> {
    let ix = create_transfer_ctoken_instruction(*from.key, *to.key, amount, *authority.key);

    invoke(&ix, &[from.clone(), to.clone(), authority.clone()])
}

/// Transfer c-tokens CPI
pub fn transfer_ctoken_signed<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = create_transfer_ctoken_instruction(*from.key, *to.key, amount, *authority.key);

    invoke_signed(
        &ix,
        &[from.clone(), to.clone(), authority.clone()],
        signer_seeds,
    )
}
