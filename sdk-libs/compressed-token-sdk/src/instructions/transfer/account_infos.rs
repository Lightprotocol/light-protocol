use arrayvec::ArrayVec;
use solana_account_info::AccountInfo;
use solana_instruction::Instruction;
use solana_msg::msg;

use crate::{account::CTokenAccount, error::Result};

pub const MAX_ACCOUNT_INFOS: usize = 20;

// TODO: test with delegate
// For pinocchio we will need to build the accounts in oder
// The easiest is probably just pass the accounts multiple times since deserialization is zero copy.
pub struct TransferAccountInfos<'a, 'info, const N: usize = MAX_ACCOUNT_INFOS> {
    pub fee_payer: &'a AccountInfo<'info>,
    pub authority: &'a AccountInfo<'info>,
    pub ctoken_accounts: &'a [AccountInfo<'info>],
    pub cpi_context: Option<&'a AccountInfo<'info>>,
    // TODO: rename tree accounts to packed accounts
    pub packed_accounts: &'a [AccountInfo<'info>],
}

impl<'info, const N: usize> TransferAccountInfos<'_, 'info, N> {
    // 874 with std::vec
    // 722 with array vec
    pub fn into_account_infos(self) -> ArrayVec<AccountInfo<'info>, N> {
        let mut capacity = 2 + self.ctoken_accounts.len() + self.packed_accounts.len();
        let ctoken_program_id_index = self.ctoken_accounts.len() - 2;
        if self.cpi_context.is_some() {
            capacity += 1;
        }

        // Check if capacity exceeds ArrayVec limit
        if capacity > N {
            panic!("Account infos capacity {} exceeds limit {}", capacity, N);
        }

        let mut account_infos = ArrayVec::<AccountInfo<'info>, N>::new();
        account_infos.push(self.fee_payer.clone());
        account_infos.push(self.authority.clone());

        // Add ctoken accounts
        for account in self.ctoken_accounts {
            account_infos.push(account.clone());
        }

        if let Some(cpi_context) = self.cpi_context {
            account_infos.push(cpi_context.clone());
        } else {
            account_infos.push(self.ctoken_accounts[ctoken_program_id_index].clone());
        }

        // Add tree accounts
        for account in self.packed_accounts {
            account_infos.push(account.clone());
        }

        account_infos
    }

    // 1528
    pub fn into_account_infos_checked(
        self,
        ix: &Instruction,
    ) -> Result<ArrayVec<AccountInfo<'info>, N>> {
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

// Note: maybe it is not useful for removing accounts results in loss of order
//       other than doing [..end] so let's just do that in the first place.
// TODO: test
/// Filter packed accounts for accounts necessary for token accounts.
/// Note accounts still need to be in the correct order.
pub fn filter_packed_accounts<'info>(
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
