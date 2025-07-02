use light_sdk_types::CTOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// Create a decompressed token transfer instruction.
/// This creates an instruction that uses discriminator 3 (DecompressedTransfer) to perform
/// SPL token transfers on decompressed compressed token accounts.
///
/// # Arguments
/// * `source` - Source token account
/// * `destination` - Destination token account
/// * `amount` - Amount to transfer
/// * `authority` - Authority pubkey
///
/// # Returns
/// `Instruction`
pub fn create_decompressed_token_transfer_instruction(
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
    authority: Pubkey,
) -> Instruction {
    Instruction {
        program_id: Pubkey::from(CTOKEN_PROGRAM_ID),
        accounts: vec![
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: {
            let mut data = vec![3u8]; // DecompressedTransfer discriminator
            data.push(3u8); // SPL Transfer discriminator
            data.extend_from_slice(&amount.to_le_bytes());
            data
        },
    }
}

/// Transfer decompressed ctokens
pub fn transfer<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
) -> Result<(), ProgramError> {
    let ix =
        create_decompressed_token_transfer_instruction(*from.key, *to.key, amount, *authority.key);

    // Return Result directly, as is best practice for CPI helpers in native Solana programs.
    invoke(&ix, &[from.clone(), to.clone(), authority.clone()])
}

/// Transfer decompressed ctokens with signer seeds
pub fn transfer_signed<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix =
        create_decompressed_token_transfer_instruction(*from.key, *to.key, amount, *authority.key);

    invoke_signed(
        &ix,
        &[from.clone(), to.clone(), authority.clone()],
        signer_seeds,
    )
}
