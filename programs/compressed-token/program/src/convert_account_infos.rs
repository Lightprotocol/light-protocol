use anchor_lang::prelude::ProgramError;
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

/// Convert Pinocchio AccountInfo to Solana AccountInfo with minimal safety overhead
///
/// # SAFETY
/// - `pinocchio_accounts` must remain valid for lifetime 'a
/// - No other code may mutably borrow these accounts during 'a
/// - Pinocchio runtime must have properly deserialized the accounts
/// - Caller must ensure no concurrent access to returned AccountInfo
#[inline(always)]
#[profile]
pub unsafe fn convert_account_infos<'a, const N: usize>(
    pinocchio_accounts: &'a [AccountInfo],
) -> Result<arrayvec::ArrayVec<anchor_lang::prelude::AccountInfo<'a>, N>, ProgramError> {
    if pinocchio_accounts.len() > N {
        return Err(ProgramError::MaxAccountsDataAllocationsExceeded);
    }

    use std::{cell::RefCell, rc::Rc};

    // Compile-time type safety: Ensure Pubkey types are layout-compatible
    const _: () = {
        assert!(
            std::mem::size_of::<pinocchio::pubkey::Pubkey>()
                == std::mem::size_of::<solana_pubkey::Pubkey>()
        );
        assert!(
            std::mem::align_of::<pinocchio::pubkey::Pubkey>()
                == std::mem::align_of::<solana_pubkey::Pubkey>()
        );
    };

    let mut solana_accounts = arrayvec::ArrayVec::<anchor_lang::prelude::AccountInfo<'a>, N>::new();
    for pinocchio_account in pinocchio_accounts {
        let key: &'a solana_pubkey::Pubkey =
            &*(pinocchio_account.key() as *const _ as *const solana_pubkey::Pubkey);

        let owner: &'a solana_pubkey::Pubkey =
            &*(pinocchio_account.owner() as *const _ as *const solana_pubkey::Pubkey);

        let lamports = Rc::new(RefCell::new(
            pinocchio_account.borrow_mut_lamports_unchecked(),
        ));

        let data = Rc::new(RefCell::new(pinocchio_account.borrow_mut_data_unchecked()));

        let account_info = anchor_lang::prelude::AccountInfo {
            key,
            lamports,
            data,
            owner,
            rent_epoch: 0, // Pinocchio doesn't track rent epoch
            is_signer: pinocchio_account.is_signer(),
            is_writable: pinocchio_account.is_writable(),
            executable: pinocchio_account.executable(),
        };

        solana_accounts.push(account_info);
    }

    Ok(solana_accounts)
}
