use solana_program::{account_info::AccountInfo, instruction::AccountMeta};

#[repr(usize)]
pub enum LightSystemAccountIndex {
    FeePayer = 0,
    Authority,
    RegisteredProgramPda,
    NoopProgram,
    AccountCompressionAuthority,
    AccountCompressionProgram,
    InvokingProgram,
    SolPoolPda,
    DecompressionRecipent,
    SystemProgram,
    LightSystemProgram,
}

pub const SYSTEM_ACCOUNTS_LEN: usize = 11;

pub struct LightCpiAccounts<'c, 'info> {
    accounts: Vec<&'c AccountInfo<'info>>,
}

impl<'c, 'info> LightCpiAccounts<'c, 'info> {
    pub fn new<I>(
        fee_payer: &'c AccountInfo<'info>,
        authority: &'c AccountInfo<'info>,
        accounts: &'c [I],
    ) -> Self
    where
        I: AsRef<AccountInfo<'info>>,
    {
        let mut cpi_accounts = Vec::with_capacity(accounts.len() + 2);
        cpi_accounts.push(fee_payer.as_ref());
        cpi_accounts.push(authority.as_ref());

        cpi_accounts.extend(accounts.into_iter().map(|acc| acc.as_ref()));

        Self {
            accounts: cpi_accounts,
        }
    }

    pub fn new_with_start_index<I>(
        fee_payer: &'c AccountInfo<'info>,
        authority: &'c AccountInfo<'info>,
        accounts: &'c [I],
        start_index: usize,
    ) -> Self
    where
        I: AsRef<AccountInfo<'info>>,
    {
        let mut cpi_accounts = Vec::with_capacity(SYSTEM_ACCOUNTS_LEN);
        cpi_accounts.push(fee_payer.as_ref());
        cpi_accounts.push(authority.as_ref());

        // `split_at` doesn't make any copies.

        // Skip the `start_index` elements.
        let (_, accounts) = accounts.split_at(start_index);
        // Take `SYSTEM_ACCOUNTS_LEN` elements, minus `fee_payer` and
        // `authority`
        let (accounts, _) = accounts.split_at(SYSTEM_ACCOUNTS_LEN - 2);
        cpi_accounts.extend(accounts.into_iter().map(|acc| acc.as_ref()));

        Self {
            accounts: cpi_accounts,
        }
    }

    pub fn fee_payer(&self) -> &'c AccountInfo<'info> {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(LightSystemAccountIndex::FeePayer as usize)
            .unwrap()
    }

    pub fn authority(&self) -> &'c AccountInfo<'info> {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(LightSystemAccountIndex::Authority as usize)
            .unwrap()
    }

    pub fn invoking_program(&self) -> &'c AccountInfo<'info> {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(LightSystemAccountIndex::InvokingProgram as usize)
            .unwrap()
    }

    pub fn light_system_program(&self) -> &'c AccountInfo<'info> {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(LightSystemAccountIndex::LightSystemProgram as usize)
            .unwrap()
    }

    #[inline(always)]
    pub fn setup_cpi_accounts(&self) -> (Vec<AccountInfo<'info>>, Vec<AccountMeta>) {
        let account_infos = self
            .accounts
            .iter()
            // Skip `light_system_program`, it shouldn't be passed as a part
            // of `account_infos` in `invoke_signed`.
            // .take(SYSTEM_ACCOUNTS_LEN - 1)
            .map(|acc| acc.as_ref().to_owned())
            .collect::<Vec<_>>();
        let account_metas = account_infos
            .iter()
            .enumerate()
            .map(|(i, acc)| {
                if i == LightSystemAccountIndex::FeePayer as usize {
                    AccountMeta {
                        pubkey: acc.key.to_owned(),
                        is_signer: true,
                        is_writable: true,
                    }
                } else if i == LightSystemAccountIndex::Authority as usize {
                    AccountMeta {
                        pubkey: acc.key.to_owned(),
                        is_signer: true,
                        is_writable: false,
                    }
                } else if i < SYSTEM_ACCOUNTS_LEN {
                    AccountMeta {
                        pubkey: acc.key.to_owned(),
                        is_signer: false,
                        is_writable: false,
                    }
                } else {
                    AccountMeta {
                        pubkey: acc.key.to_owned(),
                        is_signer: false,
                        is_writable: true,
                    }
                }
            })
            .collect::<Vec<_>>();

        (account_infos, account_metas)
    }
}
