use anchor_lang::prelude::ProgramError;
use light_program_profiler::profile;
use pinocchio::AccountView as AccountInfo;

/// Convert Pinocchio AccountInfo to Solana AccountInfo with minimal safety overhead
///
/// # SAFETY
/// - `pinocchio_accounts` must remain valid for lifetime 'a
/// - No other code may mutably borrow these accounts during 'a
/// - Pinocchio runtime must have properly deserialized the accounts
/// - Caller must ensure no concurrent access to returned AccountInfo
#[inline(always)]
#[profile]
#[allow(deprecated)]
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
            std::mem::size_of::<pinocchio::address::Address>()
                == std::mem::size_of::<solana_pubkey::Pubkey>()
        );
        assert!(
            std::mem::align_of::<pinocchio::address::Address>()
                == std::mem::align_of::<solana_pubkey::Pubkey>()
        );
    };

    let mut solana_accounts = arrayvec::ArrayVec::<anchor_lang::prelude::AccountInfo<'a>, N>::new();
    for (i, pinocchio_account) in pinocchio_accounts.iter().enumerate() {
        let key: &'a solana_pubkey::Pubkey = &*(pinocchio_account.address() as *const _);

        // For duplicate accounts, share Rc<RefCell<>> from the first occurrence
        // to prevent multiple independent mutable references to the same memory.
        // This mimics Solana runtime behavior where duplicate accounts share state.
        // SAFETY: pinocchio backs duplicate keys with the same memory region, so
        // wrapping it in a second RefCell would allow aliased &mut — hence we must
        // share the original RefCell via Rc::clone.
        if let Some(existing) = pinocchio_accounts[..i]
            .iter()
            .zip(solana_accounts.iter())
            .find(|(prev, _)| {
                light_array_map::pubkey_eq(
                    prev.address().as_array(),
                    pinocchio_account.address().as_array(),
                )
            })
            .map(|(_, acct)| acct)
        {
            solana_accounts.push(anchor_lang::prelude::AccountInfo {
                key,
                lamports: Rc::clone(&existing.lamports),
                data: Rc::clone(&existing.data),
                owner: existing.owner,
                is_signer: pinocchio_account.is_signer(),
                is_writable: pinocchio_account.is_writable(),
                executable: pinocchio_account.executable(),
                _unused: 0,
            });
            continue;
        }

        let owner: &'a solana_pubkey::Pubkey = &*(pinocchio_account.owner() as *const _);

        let lamports = Rc::new(RefCell::new(
            &mut (*pinocchio_account.account_ptr().cast_mut()).lamports,
        ));

        let data = Rc::new(RefCell::new(pinocchio_account.borrow_unchecked_mut()));

        let account_info = anchor_lang::prelude::AccountInfo {
            key,
            lamports,
            data,
            owner,
            is_signer: pinocchio_account.is_signer(),
            is_writable: pinocchio_account.is_writable(),
            executable: pinocchio_account.executable(),
            _unused: 0,
        };

        solana_accounts.push(account_info);
    }

    Ok(solana_accounts)
}
